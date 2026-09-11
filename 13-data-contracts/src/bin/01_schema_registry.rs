#![deny(warnings)]
use std::collections::HashMap;

use anyhow::Result;
use arrow::datatypes::{DataType, Field, Schema};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
enum SchemaError {
    #[error("schema version {0} already registered")]
    VersionAlreadyExists(u32),
    #[error("schema {0} não encontrado")]
    NotFound(String),
    #[error("incompatibilidade: {0}")]
    Incompatible(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchemaInfo {
    id: u32,
    nome: String,
    tipo: String,
    updated: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SchemaRegistry {
    schemas: HashMap<String, SchemaInfo>,
    next_id: u32,
}

impl SchemaRegistry {
    fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            next_id: 1,
        }
    }

    fn registrar(&mut self, nome: &str, tipo: &str) -> Result<u32, SchemaError> {
        if self.schemas.contains_key(nome) {
            return Err(SchemaError::VersionAlreadyExists(self.schemas[nome].id));
        }

        let id = self.next_id;
        self.next_id += 1;
        let info = SchemaInfo {
            id,
            nome: nome.to_string(),
            tipo: tipo.to_string(),
            updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        };
        self.schemas.insert(nome.to_string(), info);
        Ok(id)
    }

    fn buscar(&self, nome: &str) -> Result<&SchemaInfo, SchemaError> {
        self.schemas
            .get(nome)
            .ok_or_else(|| SchemaError::NotFound(nome.to_string()))
    }

    fn verificar_compatibilidade(&self, original: &str, novo: &str) -> Result<(), SchemaError> {
        let old_schema = self.buscar(original)?;
        let new_schema = self.buscar(novo)?;

        let old_fields = Self::parse_tipo_to_fields(&old_schema.tipo)?;
        let new_fields = Self::parse_tipo_to_fields(&new_schema.tipo)?;

        if old_fields.len() != new_fields.len() {
            return Err(SchemaError::Incompatible(
                "número de campos diferente".to_string(),
            ));
        }

        for (i, (old_field, new_field)) in old_fields.iter().zip(new_fields.iter()).enumerate() {
            if old_field.0 != new_field.0 {
                return Err(SchemaError::Incompatible(format!(
                    "nome do campo {} diferente: {} vs {}",
                    i, old_field.0, new_field.0
                )));
            }
            if old_field.1 != new_field.1 {
                return Err(SchemaError::Incompatible(format!(
                    "tipo do campo {} diferente: {} vs {}",
                    i, old_field.1, new_field.1
                )));
            }
        }

        Ok(())
    }

    fn parse_tipo_to_fields(tipo: &str) -> Result<Vec<(String, String)>, SchemaError> {
        let parts: Vec<&str> = tipo.splitn(2, ':').collect();
        if parts.len() < 2 {
            return Err(SchemaError::NotFound("formato inválido".to_string()));
        }

        let campos_str = parts[1];
        let mut fields = Vec::new();
        for campo_def in campos_str.split(',') {
            let field_parts: Vec<&str> = campo_def.split(':').collect();
            if field_parts.len() >= 2 {
                fields.push((field_parts[0].to_string(), field_parts[1].to_string()));
            } else {
                fields.push((field_parts[0].to_string(), "utf8".to_string()));
            }
        }
        Ok(fields)
    }

    fn para_arrow(&self, nome: &str) -> Result<Schema, SchemaError> {
        let info = self.buscar(nome)?;
        let tipo = &info.tipo;

        let parts: Vec<&str> = tipo.splitn(2, ':').collect();
        if parts.len() < 2 {
            return Err(SchemaError::NotFound(nome.to_string()));
        }

        let campos_str = parts[1];
        let mut fields = Vec::new();
        for (i, campo_def) in campos_str.split(',').enumerate() {
            let field_parts: Vec<&str> = campo_def.split(':').collect();
            let arrow_type = if field_parts.len() >= 2 {
                match field_parts[1] {
                    "i32" => DataType::Int32,
                    "f64" => DataType::Float64,
                    "utf8" => DataType::Utf8,
                    "bool" => DataType::Boolean,
                    _ => DataType::Utf8,
                }
            } else {
                DataType::Utf8
            };
            fields.push(Field::new(format!("{}", i), arrow_type, true));
        }

        Ok(Schema::new(fields))
    }
}

fn main() -> Result<()> {
    let mut registry = SchemaRegistry::new();

    println!("registro de schemas em memória");
    println!("1. registro schema 'usuarios' (id:i32, nome:utf8)");
    let id1 = registry
        .registrar("usuarios", "usuarios:id:i32,nome:utf8")
        .unwrap();
    println!("   ID: {}", id1);

    println!("2. registro schema 'pedidos' (id:i32, valor:f64)");
    let id2 = registry
        .registrar("pedidos", "pedidos:id:i32,valor:f64")
        .unwrap();
    println!("   ID: {}", id2);

    println!("3. buscar schema 'usuarios'");
    let info = registry.buscar("usuarios").unwrap();
    println!(
        "   info: {:?} (id={}, tipo={})",
        info.nome, info.id, info.tipo
    );

    println!("4. verificar compatibilidade 'usuarios' -> 'usuarios' (mesmo)");
    registry
        .verificar_compatibilidade("usuarios", "usuarios")
        .unwrap();
    println!("   OK");

    println!("5. verificar incompatibilidade 'usuarios' -> 'pedidos' (tipo diferente)");
    let err = registry
        .verificar_compatibilidade("usuarios", "pedidos")
        .unwrap_err();
    println!("   esperado: {:?}", err);

    println!(
        "Arrow Schema para 'usuarios': {:?}",
        registry.para_arrow("usuarios")
    );
    println!("\nregistros: {}", registry.schemas.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrar_novo_registra() {
        let mut reg = SchemaRegistry::new();
        let id = reg.registrar("t", "t:i32,valor:f64").unwrap();
        assert_eq!(id, 1);
        assert_eq!(reg.schemas.len(), 1);
    }

    #[test]
    fn registrar_duplicado_fails() {
        let mut reg = SchemaRegistry::new();
        reg.registrar("t", "t:i32").unwrap();
        assert!(reg.registrar("t", "t:i32").is_err());
    }

    #[test]
    fn buscar_encontra() {
        let mut reg = SchemaRegistry::new();
        reg.registrar("t", "t:i32").unwrap();
        let info = reg.buscar("t").unwrap();
        assert_eq!(info.nome, "t");
        assert_eq!(info.id, 1);
    }

    #[test]
    fn compatibilidade_pass() {
        let mut reg = SchemaRegistry::new();
        reg.registrar("a", "a:i32,valor:f64").unwrap();
        reg.registrar("b", "b:i32,valor:f64").unwrap();
        assert!(reg.verificar_compatibilidade("a", "b").is_ok());
    }

    #[test]
    fn compatibilidade_fail_tipo_dif() {
        let mut reg = SchemaRegistry::new();
        reg.registrar("a", "a:i32,valor:f64").unwrap();
        reg.registrar("b", "b:i32,nome:utf8").unwrap();
        assert!(reg.verificar_compatibilidade("a", "b").is_err());
    }

    #[test]
    fn arrow_schema() {
        let mut reg = SchemaRegistry::new();
        reg.registrar("t", "t:id:i32,nome:utf8,ativo:bool").unwrap();
        let arrow = reg.para_arrow("t").unwrap();
        assert_eq!(arrow.fields().len(), 3);
        assert_eq!(arrow.field(0).name(), "0");
    }
}
