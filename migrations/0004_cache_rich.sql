-- 0004: cache_entry 富化列（NEONLEX UI 所需的 MW 结构化字段）。
-- 旧缓存行新列均为 NULL → 读取时回落空值（serde default 兼容）。
ALTER TABLE cache_entry ADD COLUMN hom TEXT;
ALTER TABLE cache_entry ADD COLUMN meta_json TEXT;
ALTER TABLE cache_entry ADD COLUMN altprs_json TEXT;
ALTER TABLE cache_entry ADD COLUMN ins_json TEXT;
ALTER TABLE cache_entry ADD COLUMN uros_json TEXT;
ALTER TABLE cache_entry ADD COLUMN cxs_json TEXT;
