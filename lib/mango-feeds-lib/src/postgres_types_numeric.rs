use {
    bytes::{BufMut, BytesMut},
    fixed::types::I80F48,
    postgres_types::{IsNull, ToSql, Type},
    std::{cmp, error},
};

#[derive(Debug, Clone)]
pub struct SqlNumericI80F48(pub I80F48);

impl ToSql for SqlNumericI80F48 {
    fn to_sql(
        &self,
        _: &postgres_types::Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn error::Error + 'static + Sync + Send>> {
        if self.0 == 0 {
            out.reserve(10);
            out.put_u16(1); // num groups
            out.put_i16(0); // first group weight
            out.put_u16(0); // sign
            out.put_u16(0); // dscale
            out.put_i16(0); // first group
            return Ok(IsNull::No);
        }

        let abs_val = self.0.abs();
        let decimals = abs_val.int_log10();
        let first_group_weight = ((decimals as f64) / 4.0f64).floor() as i16;
        let last_group_weight = -4i16;
        let num_groups = (first_group_weight - last_group_weight + 1) as usize;

        // Reserve bytes
        out.reserve(8 + num_groups * 2);

        // Number of groups
        out.put_u16(num_groups as u16);
        // Weight of first group
        out.put_i16(first_group_weight);
        // Sign
        out.put_u16(if self.0 < 0 { 0x4000 } else { 0x0000 });
        // DScale
        out.put_u16(16);

        let mut int_part = abs_val.int().to_num::<u128>();
        let mut frac_part = (abs_val.frac() * I80F48::from_num(1e16)).to_num::<u64>();

        //info!("i80f48 {} {} {} {} {}", self.0, decimals, first_group_weight, int_part, frac_part);

        for weight in (0..=first_group_weight).rev() {
            let decimal_shift = 10000u128.pow(weight as u32);
            let v = (int_part / decimal_shift) & 0xFFFF;
            out.put_i16(v as i16);
            //info!("int {} {} {}", weight, v, int_part);
            int_part -= v * decimal_shift;
        }
        for weight in (last_group_weight..=cmp::min(first_group_weight, -1)).rev() {
            let decimal_shift = 10000u64.pow((4 + weight) as u32);
            let v = (frac_part / decimal_shift) & 0xFFFF;
            out.put_i16(v as i16);
            //info!("frac {} {} {}", weight, v, frac_part);
            frac_part -= v * decimal_shift;
        }

        Ok(IsNull::No)
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::NUMERIC)
    }

    postgres_types::to_sql_checked!();
}

// from https://github.com/rust-lang/rust/pull/86930
mod int_log {
    // 0 < val < 100_000_000
    const fn less_than_8(mut val: u32) -> u32 {
        let mut log = 0;
        if val >= 10_000 {
            val /= 10_000;
            log += 4;
        }
        log + if val >= 1000 {
            3
        } else if val >= 100 {
            2
        } else if val >= 10 {
            1
        } else {
            0
        }
    }

    // 0 < val < 10_000_000_000_000_000
    const fn less_than_16(mut val: u64) -> u32 {
        let mut log = 0;
        if val >= 100_000_000 {
            val /= 100_000_000;
            log += 8;
        }
        log + less_than_8(val as u32)
    }

    // 0 < val <= u64::MAX
    pub const fn u64(mut val: u64) -> u32 {
        let mut log = 0;
        if val >= 10_000_000_000_000_000 {
            val /= 10_000_000_000_000_000;
            log += 16;
        }
        log + less_than_16(val)
    }

    // 0 < val <= u128::MAX
    pub const fn u128(mut val: u128) -> u32 {
        let mut log = 0;
        if val >= 100_000_000_000_000_000_000_000_000_000_000 {
            val /= 100_000_000_000_000_000_000_000_000_000_000;
            log += 32;
            return log + less_than_8(val as u32);
        }
        if val >= 10_000_000_000_000_000 {
            val /= 10_000_000_000_000_000;
            log += 16;
        }
        log + less_than_16(val as u64)
    }
}

#[derive(Debug, Clone)]
pub struct SqlNumericI128(pub i128);

impl ToSql for SqlNumericI128 {
    fn to_sql(
        &self,
        _: &postgres_types::Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn error::Error + 'static + Sync + Send>> {
        let abs_val = self.0.unsigned_abs();
        let decimals = if self.0 != 0 {
            int_log::u128(abs_val)
        } else {
            0
        };
        let first_group_weight = ((decimals as f64) / 4.0f64).floor() as i16;
        let num_groups = (first_group_weight + 1) as usize;

        // Reserve bytes
        out.reserve(8 + num_groups * 2);

        // Number of groups
        out.put_u16(num_groups as u16);
        // Weight of first group
        out.put_i16(first_group_weight);
        // Sign
        out.put_u16(if self.0 < 0 { 0x4000 } else { 0x0000 });
        // DScale
        out.put_u16(0);

        let mut int_part = abs_val;

        for weight in (0..=first_group_weight).rev() {
            let decimal_shift = 10000u128.pow(weight as u32);
            let v = (int_part / decimal_shift) & 0xFFFF;
            out.put_i16(v as i16);
            int_part -= v * decimal_shift;
        }

        Ok(IsNull::No)
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::NUMERIC)
    }

    postgres_types::to_sql_checked!();
}

#[derive(Debug, Clone)]
pub struct SqlNumericU64(pub u64);

impl ToSql for SqlNumericU64 {
    fn to_sql(
        &self,
        _: &postgres_types::Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn error::Error + 'static + Sync + Send>> {
        let decimals = if self.0 != 0 { int_log::u64(self.0) } else { 0 };
        let first_group_weight = ((decimals as f64) / 4.0f64).floor() as i16;
        let num_groups = (first_group_weight + 1) as usize;

        // Reserve bytes
        out.reserve(8 + num_groups * 2);

        // Number of groups
        out.put_u16(num_groups as u16);
        // Weight of first group
        out.put_i16(first_group_weight);
        // Sign
        out.put_u16(0);
        // DScale
        out.put_u16(0);

        let mut int_part = self.0;

        for weight in (0..=first_group_weight).rev() {
            let decimal_shift = 10000u64.pow(weight as u32);
            let v = (int_part / decimal_shift) & 0xFFFF;
            out.put_i16(v as i16);
            int_part -= v * decimal_shift;
        }

        Ok(IsNull::No)
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::NUMERIC)
    }

    postgres_types::to_sql_checked!();
}
