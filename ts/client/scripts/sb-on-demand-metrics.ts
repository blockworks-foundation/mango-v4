import express from "express";
import * as prom from "prom-client";
import promBundle from "express-prom-bundle";

const collectDefaultMetrics = prom.collectDefaultMetrics;
collectDefaultMetrics({
  labels: {
    app: process.env.FLY_APP_NAME,
    instance: process.env.FLY_ALLOC_ID,
  },
});

export const refreshSuccessCounter = new prom.Counter({
  name: "refresh_success_count",
  help: "number of successful refreshes",
  labelNames: ["label"],
});

export const refreshFailureCounter = new prom.Counter({
  name: "refresher_failure_count",
  help: "number of failed refreshes",
  labelNames: ["err"],
});

export const relativeSlotsSinceLastUpdateHistogram = new prom.Histogram({
  name: "relative_slots_since_last_update",
  help: "distribution of the relative slot since last update during filterForStaleOracles",
  labelNames: ["oracle"],
  buckets: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.75, 2.0, 3.0, 4.0, 5.0],
});

export const relativeVarianceSinceLastUpdateHistogram = new prom.Histogram({
  name: "relative_variance_since_last_update",
  help: "distribution of the relative variance since last update during filterForVarianceThresholdOracles",
  labelNames: ["oracle"],
  buckets: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.75, 2.0, 3.0, 4.0, 5.0],
});

export const fetchIxSuccessCounter = new prom.Counter({
  name: "fetch_ix_success_count",
  help: "number of successful oracle instructions fetched",
  labelNames: ["oracle"],
});

export const fetchIxFailureCounter = new prom.Counter({
  name: "fetch_ix_failure_count",
  help: "number of failed ix fetches",
  labelNames: ["err"],
});

export const fetchIxDurationHistogram = new prom.Histogram({
  name: "fetch_ix_duration_histogram",
  help: "duration of fetchIx calls in ms",
  buckets: [100, 200, 400, 800, 1_600, 3_200, 6_400, 12_800, 25_600, 51_200, 102_400],
});


export const simulateFeedDurationHistogram = new prom.Histogram({
  name: "simulate_feed_duration_histogram",
  help: "duration of simulateFeeds calls in ms",
  buckets: [100, 200, 400, 800, 1_600, 3_200, 6_400, 12_800, 25_600, 51_200, 102_400],
});

export const updateOracleAiDurationHistogram = new prom.Histogram({
  name: "update_oracle_ai_duration_histogram",
  help: "duration of updateFilteredOraclesAis calls in ms",
  buckets: [100, 200, 400, 800, 1_600, 3_200, 6_400, 12_800, 25_600, 51_200, 102_400],
});

export const updateBlockhashSlotDurationHistogram = new prom.Histogram({
  name: "update_blockhash_slot_duration_histogram",
  help: "duration of getLatestBlockhash+getSlot calls in ms",
  buckets: [100, 200, 400, 800, 1_600, 3_200, 6_400, 12_800, 25_600, 51_200, 102_400],
});

export const cycleDurationHistogram = new prom.Histogram({
  name: "cycle_duration_histogram",
  help: "duration of full update cycle in ms",
  buckets: [100, 200, 400, 800, 1_600, 3_200, 6_400, 12_800, 25_600, 51_200, 102_400],
});

export const sendTxCounter = new prom.Counter({
  name: "send_tx_counter",
  help: "number of oracle update transactions sent",
});

export const sendTxErrorCounter = new prom.Counter({
  name: "send_tx_error_counter",
  help: "number of failed oracle update transactions",
  labelNames: ["err"],
});


export const metricsMiddleware = promBundle({ includeMethod: true });


export function exposePromMetrics(port: number, bind: string | undefined): void {
  const app = express();
  app.use(metricsMiddleware);
  app.listen(port, bind, () => {
    console.log(`prom metrics exposed on ${bind}:${port}`);
  });
}

