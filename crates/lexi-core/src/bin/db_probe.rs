//! 调试工具（非产品代码）：检查已导入词库与查询链路。
//!
//! 用法: cargo run -p lexi-core --bin db_probe -- [db_path]

use std::sync::Arc;

use lexi_core::domain::normalizer::Normalizer;
use lexi_core::infra::db::{CedictRepo, Db, EcdictRepo};
use lexi_core::infra::mw::parse_body;

fn main() -> anyhow::Result<()> {
    let db_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/debug/data/db/lexi.sqlite".to_string());
    let db = Arc::new(Db::open(std::path::Path::new(&db_path))?);

    let ecdict = EcdictRepo::new(db.clone());
    let cedict = CedictRepo::new(db.clone());

    println!("ecdict rows: {}", ecdict.count()?);
    println!("cedict rows: {}", cedict.count()?);

    // 英文兜底链
    for word in ["apple", "voluminous", "the", "test"] {
        match ecdict.lookup(word)? {
            Some(r) => println!(
                "  {word}: phonetic={} translation={:?} tag={} frq={}",
                r.phonetic,
                r.translation.lines().next().unwrap_or(""),
                r.tag,
                r.frq
            ),
            None => println!("  {word}: MISSING"),
        }
    }

    // 中文链
    let n = Normalizer::l1();
    for zh in ["你好", "中国", "电脑"] {
        let q = n.normalize(zh);
        let exact = cedict.lookup_exact(&q.key)?;
        println!(
            "  {zh}: exact={:?}",
            exact
                .first()
                .map(|r| format!("{}/{} [{}] {}", r.traditional, r.simplified, r.pinyin, r.english))
        );
    }
    let py = cedict.search_pinyin("ni ha", 3)?;
    println!("  pinyin 'ni ha': {} hits (first: {:?})", py.len(), py.first().map(|r| r.simplified.clone()));
    let fts = cedict.search_english("computer", 3)?;
    println!("  english 'computer': {} hits", fts.len());
    let zh_fts = ecdict.search_translation("苹果", 3)?;
    println!("  ecdict zh '苹果': {:?}",
        zh_fts.iter().map(|(w, t)| format!("{w}:{}", t.chars().take(20).collect::<String>())).collect::<Vec<_>>());

    // MW 解析冒烟（文档样例）
    let body = r#"[{"meta":{"id":"x","stems":["x"]},"hwi":{"hw":"x","prs":[{"mw":"eks","sound":{"audio":"x0001","ref":"c","stat":"1"}}]},"fl":"noun","shortdef":["s"],"def":[{"sseq":[[["sense",{"sn":"1","dt":[["text","{bc}s"]]}]]]}]}]"#;
    match parse_body(body)? {
        lexi_core::infra::mw::MwResponse::Entries(es) => {
            println!("mw parse smoke: headword={} audio={:?}", es[0].headword, es[0].audio_codes);
        }
        _ => println!("mw parse smoke: unexpected"),
    }
    Ok(())
}
