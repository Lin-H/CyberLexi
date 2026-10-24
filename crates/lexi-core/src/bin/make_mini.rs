//! 数据工具（非产品代码）：从完整版 ECDICT csv 生成 mini 子集。
//!
//! 仓库中的 `assets/dict/ecdict.mini.csv` 由本工具生成；筛选规则：
//! 保留 frq/bnc/collins/oxford 任一 > 0 或 tag 非空的词条（即有词频
//! 或考试标注的核心词汇），与 ECDICT 官方 mini 的取材思路一致。
//!
//! 用法: cargo run -p lexi-core --bin make_mini -- <full.csv> <out.csv>

use csv::{ReaderBuilder, WriterBuilder};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (input, output) = match (args.get(1), args.get(2)) {
        (Some(i), Some(o)) => (i.clone(), o.clone()),
        _ => anyhow::bail!("usage: make_mini <full.csv> <out.csv>"),
    };

    let mut rdr = ReaderBuilder::new().flexible(true).from_path(&input)?;
    let headers = rdr.headers()?.clone();
    let col = |name: &str| -> anyhow::Result<usize> {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| anyhow::anyhow!("missing column: {name}"))
    };
    let (i_frq, i_bnc, i_collins, i_oxford, i_tag) =
        (col("frq")?, col("bnc")?, col("collins")?, col("oxford")?, col("tag")?);

    let mut wtr = WriterBuilder::new().from_path(&output)?;
    wtr.write_record(&headers)?;
    let (mut kept, mut total) = (0u64, 0u64);
    for rec in rdr.records() {
        let rec = rec?;
        total += 1;
        let num = |i: usize| {
            rec.get(i)
                .and_then(|s| s.trim().parse::<i64>().ok())
                .unwrap_or(0)
        };
        let keep = num(i_frq) > 0
            || num(i_bnc) > 0
            || num(i_collins) > 0
            || num(i_oxford) > 0
            || rec.get(i_tag).is_some_and(|t| !t.trim().is_empty());
        if keep {
            wtr.write_record(&rec)?;
            kept += 1;
        }
    }
    wtr.flush()?;
    println!("total={total} kept={kept} -> {output}");
    Ok(())
}
