-- Sprint 2：向量记忆检索（pgvector；不定维度以适配不同 embedding 模型，小规模暴力余弦）
CREATE EXTENSION IF NOT EXISTS vector;
ALTER TABLE memories ADD COLUMN IF NOT EXISTS embedding vector;
