//! Módulo 09 — Visualização de Dados
//! README: seção "Exemplo prático 2: Dashboard interativo"
//!
//! Dashboard `egui` que carrega dados do Módulo 2 e permite filtrar/explorar
//! interativamente com sliders e dropdowns. Em produção, seria usado como tool
//! interna para monitoramento.

use anyhow::Result;
use eframe::egui;
use polars::prelude::*;
use std::path::Path;

#[derive(Debug, Clone)]
struct Registro {
    data: String,
    metrica: f64,
    categoria: String,
}

struct DashboardApp {
    dados: Vec<Registro>,
    filtro_categoria: String,
    min_metrica: f64,
    max_metrica: f64,
    threshold: f64,
}

impl DashboardApp {
    fn new(dados: Vec<Registro>) -> Self {
        let min = dados
            .iter()
            .map(|r| r.metrica)
            .fold(f64::INFINITY, f64::min);
        let max = dados
            .iter()
            .map(|r| r.metrica)
            .fold(f64::NEG_INFINITY, f64::max);
        Self {
            dados,
            filtro_categoria: "todas".to_string(),
            min_metrica: min,
            max_metrica: max,
            threshold: (min + max) / 2.0,
        }
    }

    fn filtrados(&self) -> Vec<&Registro> {
        self.dados
            .iter()
            .filter(|r| {
                (self.filtro_categoria == "todas" || r.categoria == self.filtro_categoria)
                    && r.metrica >= self.threshold
                    && r.metrica >= self.min_metrica
                    && r.metrica <= self.max_metrica
            })
            .collect()
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Dashboard — Visualização de Dados (Módulo 09)");
            ui.label(format!(
                "Total registros: {} | Filtrados: {}",
                self.dados.len(),
                self.filtrados().len()
            ));
        });

        egui::SidePanel::left("filtros").show(ctx, |ui| {
            ui.heading("Filtros");
            let categorias = ["todas", "A", "B", "C"];
            egui::ComboBox::from_label("Categoria")
                .selected_text(&self.filtro_categoria)
                .show_ui(ui, |ui| {
                    for cat in categorias {
                        ui.selectable_value(&mut self.filtro_categoria, cat.to_string(), cat);
                    }
                });

            ui.add(
                egui::Slider::new(&mut self.threshold, self.min_metrica..=self.max_metrica)
                    .text("Threshold metrica"),
            );
            ui.add(egui::Slider::new(&mut self.min_metrica, 0.0..=200.0).text("Min"));
            ui.add(egui::Slider::new(&mut self.max_metrica, 0.0..=500.0).text("Max"));

            if ui.button("Reset").clicked() {
                *self = DashboardApp::new(self.dados.clone());
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Dados filtrados");
            let filtrados = self.filtrados();
            // Tabela simples
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("tabela")
                    .num_columns(3)
                    .spacing([20.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Data").strong());
                        ui.label(egui::RichText::new("Categoria").strong());
                        ui.label(egui::RichText::new("Metrica").strong());
                        ui.end_row();
                        for r in filtrados.iter().take(50) {
                            ui.label(&r.data);
                            ui.label(&r.categoria);
                            ui.label(format!("{:.2}", r.metrica));
                            ui.end_row();
                        }
                    });
            });

            // Gráfico simples com barras via egui (sem plotters, nativo egui)
            ui.separator();
            ui.label("Distribuição (threshold vs metrica)");
            let max = filtrados
                .iter()
                .map(|r| r.metrica)
                .fold(0.0, f64::max)
                .max(1.0);
            for r in filtrados.iter().take(10) {
                let w = (r.metrica / max * 200.0) as usize;
                ui.horizontal(|ui| {
                    ui.label(&r.categoria);
                    ui.add(
                        egui::ProgressBar::new((r.metrica / max) as f32)
                            .text(format!("{:.1}", r.metrica)),
                    );
                });
                let _ = w;
            }
        });
    }
}

fn carregar_dados(path: &Path) -> Result<Vec<Registro>> {
    let df = if path.exists() {
        LazyCsvReader::new(path.to_string_lossy().as_ref().into())
            .with_has_header(true)
            .finish()
            .map_err(|e| anyhow::anyhow!("csv: {e}"))?
            .collect()
            .map_err(|e| anyhow::anyhow!("collect: {e}"))?
    } else {
        // Sintético
        let mut dados = Vec::new();
        for i in 0..30 {
            dados.push(Registro {
                data: format!("2024-01-{:02}", (i % 30) + 1),
                metrica: 100.0 + (i as f64 * 0.7).sin() * 30.0 + i as f64,
                categoria: ["A", "B", "C"][i % 3].to_string(),
            });
        }
        return Ok(dados);
    };

    // Tenta extrair colunas do Módulo 2
    let mut registros = Vec::new();
    let n = df.height();
    // Tenta timestamp/metrica, senão usa genérico
    if df.column("timestamp").is_ok() && df.column("metrica").is_ok() {
        let ts = df.column("timestamp").unwrap();
        let met = df.column("metrica").unwrap().f64().unwrap();
        for i in 0..n {
            let data = ts
                .get(i)
                .map(|v| format!("{v:?}"))
                .unwrap_or_else(|_| format!("row-{i}"));
            let metrica = met.get(i).unwrap_or(0.0);
            registros.push(Registro {
                data,
                metrica,
                categoria: if metrica > 120.0 {
                    "A".to_string()
                } else {
                    "B".to_string()
                },
            });
        }
    } else {
        for i in 0..n {
            registros.push(Registro {
                data: format!("row-{i}"),
                metrica: 100.0 + i as f64,
                categoria: "A".to_string(),
            });
        }
    }
    Ok(registros)
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../02-processamento-dados/dados/series_temporais.csv".to_string());
    let dados = carregar_dados(Path::new(&path))?;
    println!(
        "Dashboard carregado com {} registros de {}",
        dados.len(),
        path
    );

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Dashboard Módulo 09",
        options,
        Box::new(|_cc| Ok(Box::new(DashboardApp::new(dados)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carregar_sintetico() {
        let dados = carregar_dados(Path::new("/tmp/nao_existe.csv")).expect("ok");
        assert!(!dados.is_empty());
    }

    #[test]
    fn filtro_funciona() {
        let dados = vec![
            Registro {
                data: "2024-01-01".to_string(),
                metrica: 100.0,
                categoria: "A".to_string(),
            },
            Registro {
                data: "2024-01-02".to_string(),
                metrica: 200.0,
                categoria: "B".to_string(),
            },
        ];
        let mut app = DashboardApp::new(dados);
        app.threshold = 150.0;
        let f = app.filtrados();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].categoria, "B");
    }
}
