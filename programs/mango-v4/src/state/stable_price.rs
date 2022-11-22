use crate::util::checked_math as cm;
use anchor_lang::prelude::*;
use derivative::Derivative;
use static_assertions::const_assert_eq;
use std::mem::size_of;

/// Maintains a "stable_price" based on the oracle price.
///
/// The stable price follows the oracle price, but its relative rate of
/// change is limited (to `stable_growth_limit`) and futher reduced if
/// the oracle price is far from the `delay_price`.
///
/// Conceptually the `delay_price` is itself a time delayed
/// (`24 * delay_interval_seconds`, assume 24h) and relative rate of change limited
/// function of the oracle price. It is implemented as averaging the oracle
/// price over every `delay_interval_seconds` (assume 1h) and then applying the
/// `delay_growth_limit` between intervals.
#[zero_copy]
#[derive(Derivative, Debug)]
pub struct StablePriceModel {
    /// Current stable price to use in health
    pub stable_price: f64,

    /// Stored delay_price for each delay_interval.
    /// If we want the delay_price to be 24h delayed, we would store one for each hour.
    /// This is used in a cyclical way: We use the maximally-delayed value at delay_interval_index
    /// and once enough time passes to move to the next delay interval, that gets overwritten and
    /// we use the next one.
    pub delay_prices: [f64; 24],

    /// The delay price is based on an average over each delay_interval. The contributions
    /// to the average are summed up here.
    pub delay_accumulator_price: f64,

    /// Accumulating the total time for the above average.
    pub delay_accumulator_time: u32,

    /// Length of a delay_interval
    pub delay_interval_seconds: u32,

    /// Maximal relative difference between two delay_price in consecutive intervals.
    pub delay_growth_limit: f32,

    /// Maximal per-second relative difference of the stable price.
    /// It gets further reduced if stable and delay price disagree.
    pub stable_growth_limit: f32,

    /// The delay_interval_index that update() was last called on.
    pub last_delay_interval_index: u8,

    #[derivative(Debug = "ignore")]
    pub padding: [u8; 7],
}
const_assert_eq!(size_of::<StablePriceModel>(), 232);
const_assert_eq!(size_of::<StablePriceModel>() % 8, 0);

impl Default for StablePriceModel {
    fn default() -> Self {
        Self {
            stable_price: 0.0,
            delay_prices: [0.0; 24],
            delay_accumulator_price: 0.0,
            delay_accumulator_time: 0,
            delay_interval_seconds: 60 * 60, // 1h, for a total delay of 24h
            delay_growth_limit: 0.06,        // 6% per hour, 400% per day
            stable_growth_limit: 0.0003, // 0.03% per second, 293% in 1h if updated every 10s, 281% in 1h if updated every 5min
            last_delay_interval_index: 0,
            padding: Default::default(),
        }
    }
}

impl StablePriceModel {
    pub fn reset_to_price(&mut self, oracle_price: f64) {
        self.stable_price = oracle_price;
        self.delay_prices = [oracle_price; 24];
        self.delay_accumulator_price = 0.0;
        self.delay_accumulator_time = 0;
    }

    pub fn delay_interval_index(&self, timestamp: u64) -> u8 {
        // TODO: check compute costs
        ((timestamp / self.delay_interval_seconds as u64) % self.delay_prices.len() as u64) as u8
    }

    pub fn update(&mut self, timestamp: u64, last_update: u64, oracle_price: f64) {
        let dt = timestamp.saturating_sub(last_update);
        // Hardcoded. Requiring a minimum time between updates reduces the possible difference
        // between frequent updates and infrequent ones.
        // Limiting the max dt prevents very strong updates if update() hasn't been
        // called for hours.
        let min_dt = 10;
        let max_dt = 10 * 60; // 10 min
        if dt < min_dt {
            return;
        }
        let dt = dt.min(max_dt) as f64;

        //
        // Update delay price
        //
        cm!(self.delay_accumulator_time += dt as u32);
        self.delay_accumulator_price += oracle_price * dt;

        let delay_interval_index = self.delay_interval_index(timestamp);
        if delay_interval_index != self.last_delay_interval_index {
            let new_delay_price = {
                let prev = if self.last_delay_interval_index == 0 {
                    self.delay_prices[self.delay_prices.len() - 1]
                } else {
                    self.delay_prices[self.last_delay_interval_index as usize - 1]
                };
                if self.delay_accumulator_time == 0 {
                    prev
                } else {
                    let avg = self.delay_accumulator_price / (self.delay_accumulator_time as f64);
                    let max = prev * (1.0 + self.delay_growth_limit as f64);
                    let min = prev * (1.0 - self.delay_growth_limit as f64);
                    if avg > max {
                        max
                    } else if avg < min {
                        min
                    } else {
                        avg
                    }
                }
            };

            // Store the new delay price, accounting for skipped intervals
            // TODO: this breaks if we're skipping more than 24 intervals...
            if delay_interval_index > self.last_delay_interval_index {
                for i in self.last_delay_interval_index..delay_interval_index {
                    self.delay_prices[i as usize] = new_delay_price;
                }
            } else {
                for i in self.last_delay_interval_index as usize..self.delay_prices.len() {
                    self.delay_prices[i] = new_delay_price;
                }
                for i in 0..delay_interval_index {
                    self.delay_prices[i as usize] = new_delay_price;
                }
            }

            self.delay_accumulator_price = 0.0;
            self.delay_accumulator_time = 0;
            self.last_delay_interval_index = delay_interval_index;
        }

        let delay_price = self.delay_prices[delay_interval_index as usize];

        //
        // Update stable price
        //
        let prev_stable_price = self.stable_price;
        let fraction = if delay_price >= prev_stable_price {
            prev_stable_price / delay_price
        } else {
            delay_price / prev_stable_price
        };
        let growth_limit = (self.stable_growth_limit as f64) * fraction * fraction * dt;
        // for the lower bound, we technically should divide by (1 + growth_limit), but
        // the error is small when growth_limit is small and this saves a division
        let lower_bound = prev_stable_price * (1.0 - growth_limit);
        let upper_bound = prev_stable_price * (1.0 + growth_limit);
        self.stable_price = if oracle_price < lower_bound {
            lower_bound
        } else if oracle_price > upper_bound {
            upper_bound
        } else {
            oracle_price
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_and_print(
        model: &mut StablePriceModel,
        start: u64,
        dt: u64,
        steps: u64,
        price: fn(u64) -> f64,
    ) -> u64 {
        println!("step,timestamp,stable_price,delay_price");
        for i in 0..steps {
            let time = start + dt * (i + 1);
            model.update(time, time - dt, price(time));
            println!(
                "{i},{time},{},{}",
                model.stable_price, model.delay_prices[model.last_delay_interval_index as usize]
            );
        }
        start + dt * steps
    }

    #[test]
    fn test_stable_price_10x() {
        let mut model = StablePriceModel::default();
        model.reset_to_price(1.0);

        let mut t;
        t = run_and_print(&mut model, 0, 60, 60, |_| 10.0);
        assert!((model.stable_price - 1.8).abs() < 0.1);
        assert_eq!(model.delay_prices[1..], [1.0; 23]);
        assert!((model.delay_prices[0] - 1.06).abs() < 0.01);
        assert_eq!(model.last_delay_interval_index, 1);
        assert_eq!(model.delay_accumulator_time, 0);
        assert_eq!(model.delay_accumulator_price, 0.0);

        t = run_and_print(&mut model, t, 10, 6 * 60, |_| 10.0);
        assert!((model.stable_price - 2.3).abs() < 0.1);
        assert_eq!(model.delay_prices[2..], [1.0; 22]);
        assert!((model.delay_prices[0] - 1.06).abs() < 0.01);
        assert!((model.delay_prices[1] - 1.06 * 1.06).abs() < 0.01);
        assert_eq!(model.last_delay_interval_index, 2);
        assert_eq!(model.delay_accumulator_time, 0);
        assert_eq!(model.delay_accumulator_price, 0.0);

        // check delay price wraparound (go to 25h since start)
        t = run_and_print(&mut model, t, 300, 12 * 23, |_| 10.0);
        assert!((model.stable_price - 7.4).abs() < 0.1);
        assert!(model.delay_prices[0] > model.delay_prices[23]);
        assert!(model.delay_prices[23] > model.delay_prices[22]);
        assert!(model.delay_prices[1] < model.delay_prices[0]);
        assert!(model.delay_prices[1] < model.delay_prices[2]);
        assert_eq!(model.last_delay_interval_index, 1);

        println!("{t}");
    }

    #[test]
    fn test_stable_price_characteristics_upwards() {
        let mut model = StablePriceModel::default();

        model.reset_to_price(1.0);

        let mut last = 1;
        for i in 0..100000 {
            model.update(60 * (i + 1), 60 * i, 1000.0);
            let now = model.stable_price as i32;
            if now > last {
                last = now;
                println!("reached {now}x after {i} steps, {} hours", i as f64 / 60.0);
                if now == 10 {
                    break;
                }
            }
        }
    }

    #[test]
    fn test_stable_price_characteristics_downwards() {
        let mut model = StablePriceModel::default();
        let init = 10000.0;
        model.reset_to_price(init);

        let mut last = 1;
        for i in 0..100000 {
            model.update(60 * (i + 1), 60 * i, 0.0);
            let now = (init / model.stable_price) as i32;
            if now > last {
                last = now;
                println!(
                    "reached 1/{now}x after {i} steps, {} hours",
                    i as f64 / 60.0
                );
                if now == 10 {
                    break;
                }
            }
        }
    }

    #[test]
    fn test_stable_price_average() {
        let mut model = StablePriceModel {
            delay_growth_limit: 10.00,
            ..StablePriceModel::default()
        };
        model.reset_to_price(1.0);

        run_and_print(&mut model, 0, 60, 60, |t| if t > 1800 { 2.0 } else { 1.0 });
        println!("{}", model.delay_prices[0]);
        assert!((model.delay_prices[0] - 1.5).abs() < 0.01);
    }
}
