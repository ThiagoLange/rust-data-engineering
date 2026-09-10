//! Módulo 09 — Visualização de Dados
//! README: seção "Exercício" — Dashboard streaming
//!
//! Estende o dashboard `egui` para consumir dados em tempo real do pipeline
//! de streaming do Módulo 4 via `tokio::sync::mpsc`, atualizando gráficos
//! conforme novos eventos chegam.

use anyhow::Result;
use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
struct EventoStream {
    timestamp: String,
    valor: f64,
    categoria: String,
}

struct StreamingDashboard {
    dados: Arc<Mutex<Vec<EventoStream>>>,
    rx: Option<mpsc::Receiver<EventoStream>>,
    threshold: f64,
    filtro: String,
}

impl eframe::App for StreamingDashboard {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Tenta receber novos eventos sem bloquear (poll)
        if let Some(rx) = &mut self.rx {
            while let Ok(ev) = rx.try_recv() {
                self.dados.lock().unwrap().push(ev);
                ctx.request_repaint();
            }
        }
        // Força repaint para animação
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        let dados = self.dados.lock().unwrap().clone();
        let filtrados: Vec<EventoStream> = dados
            .iter()
            .filter(|e| {
                (self.filtro == "todas" || e.categoria == self.filtro) && e.valor >= self.threshold
            })
            .cloned()
            .collect();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Dashboard Streaming — Módulo 09 + 04");
            ui.label(format!(
                "Total: {} | Filtrados: {} | Threshold: {:.1}",
                dados.len(),
                filtrados.len(),
                self.threshold
            ));
            ui.label(
                "Novos eventos chegam via mpsc channel (simulando Kafka → Parquet → Dashboard)",
            );
        });

        egui::SidePanel::left("filtros").show(ctx, |ui| {
            ui.heading("Filtros");
            egui::ComboBox::from_label("Categoria")
                .selected_text(&self.filtro)
                .show_ui(ui, |ui| {
                    for cat in ["todas", "A", "B", "C"] {
                        ui.selectable_value(&mut self.filtro, cat.to_string(), cat);
                    }
                });
            ui.add(egui::Slider::new(&mut self.threshold, 0.0..=200.0).text("Threshold"));
            if ui.button("Limpar").clicked() {
                self.dados.lock().unwrap().clear();
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Eventos em tempo real");
            // Gráfico de barras simples
            for ev in filtrados.iter().rev().take(20) {
                let max = filtrados
                    .iter()
                    .map(|e| e.valor)
                    .fold(0.0, f64::max)
                    .max(1.0);
                ui.horizontal(|ui| {
                    ui.label(format!("{} [{}]", ev.timestamp, ev.categoria));
                    ui.add(
                        egui::ProgressBar::new((ev.valor / max) as f32)
                            .text(format!("{:.1}", ev.valor)),
                    );
                });
            }
            ui.separator();
            ui.label(format!(
                "Últimos {} eventos (de {} total)",
                filtrados.len().min(20),
                filtrados.len()
            ));
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Dashboard Streaming — Módulo 09 Exercício ===\n");
    println!("Simula pipeline Módulo 4 (Kafka) → channel Tokio → egui");

    let dados = Arc::new(Mutex::new(Vec::new()));
    let (tx, rx) = mpsc::channel(100);

    // Tarefa que simula produtor streaming (como em 04-streaming)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
        let mut id = 0;
        loop {
            interval.tick().await;
            let ev = EventoStream {
                timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                valor: 50.0 + (id as f64 * 0.3).sin() * 50.0 + rand::random::<f64>() * 20.0,
                categoria: ["A", "B", "C"][id % 3].to_string(),
            };
            if tx.send(ev).await.is_err() {
                break;
            }
            id += 1;
        }
    });

    // Para_rand precisa de rand crate — usamos fastrand se não tiver
    // Aqui usamos rand via `::rand` já em Cargo.toml? Adicionamos `rand` como deps
    let dados_clone = dados.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 600.0]),
        ..Default::default()
    };

    // eframe::run_native bloqueia, então rodamos em thread separada
    // Simplificamos: rodamos egui no thread principal, mas como já estamos em tokio runtime,
    // usamos `tokio::task::spawn_blocking` não é necessário — eframe cria seu próprio runtime
    // Para demo, saímos do tokio context e rodamos eframe
    drop(tokio::runtime::Handle::try_current());

    eframe::run_native(
        "Dashboard Streaming",
        options,
        Box::new(|_cc| {
            Ok(Box::new(StreamingDashboard {
                dados: dados_clone,
                rx: Some(rx),
                threshold: 50.0,
                filtro: "todas".to_string(),
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtro_streaming() {
        let dados = Arc::new(Mutex::new(vec![
            EventoStream {
                timestamp: "00:00:01".to_string(),
                valor: 100.0,
                categoria: "A".to_string(),
            },
            EventoStream {
                timestamp: "00:00:02".to_string(),
                valor: 30.0,
                categoria: "B".to_string(),
            },
        ]));
        let dash = StreamingDashboard {
            dados: dados.clone(),
            rx: None,
            threshold: 50.0,
            filtro: "todas".to_string(),
        };
        // threshold 50 deve filtrar 1
        let filtrados: Vec<EventoStream> = dados
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.valor >= dash.threshold)
            .cloned()
            .collect();
        assert_eq!(filtrados.len(), 1);
    }
}
