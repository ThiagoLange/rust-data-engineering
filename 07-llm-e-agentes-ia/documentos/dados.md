# Formatos de Dados

Parquet é colunar e comprimido, ideal para analytics. Avro é orientado a linha, bom para streaming. Arrow IPC é para transferência entre processos com zero-copy.

## Lakehouse

Delta Lake e Iceberg adicionam transações ACID sobre object storage. AI Lake combina Parquet com índice vetorial HNSW no footer.
