CREATE SCHEMA IF NOT EXISTS mango_monitoring AUTHORIZATION CURRENT_ROLE;
CREATE TABLE IF NOT EXISTS mango_monitoring.health_history
(
    Pubkey VARCHAR(44) NOT NULL,
    Timestamp TIMESTAMP NOT NULL,
    HealthRatio DOUBLE PRECISION
);
CREATE TABLE IF NOT EXISTS mango_monitoring.health_current
(
    Pubkey VARCHAR(44) NOT NULL PRIMARY KEY,
    Timestamp TIMESTAMP NOT NULL,
    HealthRatio DOUBLE PRECISION
);
CREATE INDEX health_history_index ON mango_monitoring.health_history
(
 Timestamp ASC
);