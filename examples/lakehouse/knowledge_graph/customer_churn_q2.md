---
id: "customer_churn_q2"
type: "external_asset"
source: "s3://lake/churn_data.parquet"
importance: 8
links: ["retention_strategy_01"]
---
# Customer Churn Q2 Analysis

This node points at external lakehouse data. Use `LakehouseReader` to resolve the `source` URI at read time.
