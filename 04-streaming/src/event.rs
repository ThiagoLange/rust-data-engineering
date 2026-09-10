//! Tipos compartilhados para o pipeline Kafka → Parquet.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Evento JSON que trafega no Kafka.
/// Mantido simples para ser gerado sinteticamente pelo produtor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub user_id: u64,
    pub event_type: String,
    pub value: f64,
    /// Campo derivado na transformação — ex: `value * 1.1`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_transformed: Option<f64>,
}

impl Event {
    /// Transformação de exemplo: aplica markup de 10% e normaliza `event_type`.
    pub fn transform(mut self) -> Self {
        self.value_transformed = Some(self.value * 1.1);
        self.event_type = self.event_type.to_lowercase();
        self
    }

    /// Partição por data (YYYY-MM-DD) para o Lake (ex: `date=2024-01-01`).
    pub fn partition_date(&self) -> String {
        self.timestamp.format("%Y-%m-%d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn transform_aplica_markup() {
        let e = Event {
            id: 1,
            timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            user_id: 42,
            event_type: "PURCHASE".to_string(),
            value: 100.0,
            value_transformed: None,
        };
        let t = e.transform();
        let v = t.value_transformed.expect("deve ter valor");
        assert!((v - 110.0).abs() < 1e-9);
        assert_eq!(t.event_type, "purchase");
    }

    #[test]
    fn partition_date_formata_corretamente() {
        let e = Event {
            id: 1,
            timestamp: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
            user_id: 1,
            event_type: "view".to_string(),
            value: 1.0,
            value_transformed: None,
        };
        assert_eq!(e.partition_date(), "2024-06-15");
    }
}
