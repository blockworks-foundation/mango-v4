CREATE SCHEMA IF NOT EXISTS mango_monitoring AUTHORIZATION CURRENT_ROLE;
CREATE TABLE IF NOT EXISTS mango_monitoring.health_history
(
    Pubkey VARCHAR(44) NOT NULL,
    Timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    MaintenanceRatio DOUBLE PRECISION,
    Maintenance DOUBLE PRECISION,
    Initial DOUBLE PRECISION,
    LiquidationEnd DOUBLE PRECISION,
    IsBeingLiquidated BOOLEAN
);
CREATE MATERIALIZED VIEW mango_monitoring.health_current AS
    SELECT DISTINCT ON (pubkey)
    *
    FROM mango_monitoring.health_history
    ORDER BY pubkey, timestamp DESC;

CREATE INDEX health_history_pubkey_index ON mango_monitoring.health_history
(
 Pubkey ASC,
 Timestamp ASC
);
CREATE INDEX health_history_timestamp_index ON mango_monitoring.health_history
(
 Timestamp ASC
);