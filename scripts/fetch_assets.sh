#!/usr/bin/env bash
# CyberLexi asset fetcher —— 数据与字体获取/再生成脚本
#
# 用法：bash scripts/fetch_assets.sh
# 产物：
#   assets/dict/ecdict.mini.csv   ECDICT 子集（MIT）
#   assets/dict/cedict.txt        CC-CEDICT 官方导出（CC BY-SA 4.0）
#   assets/fonts/*.ttf|otf        JetBrains Mono + Noto Sans SC（OFL）
#
# ECDICT mini 生成链路（官方 repo 的 mini 已缩为示例文件）：
#   npm registry tarball（完整 csv）→ make_mini 工具筛选（frq/bnc/collins/
#   oxford/tag 任一非空）→ 约 5.9 万行 / 10.4MB，与官方 mini 规模一致。

set -euo pipefail
cd "$(dirname "$0")/.."
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

DL() { curl -sL --retry 3 --max-time 600 "$1" -o "$2"; echo "OK $2 ($(stat -c%s "$2") bytes)"; }

echo "== 1/4 CC-CEDICT =="
DL "https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz" "$TMP/cedict.gz"
gunzip -f "$TMP/cedict.gz" && mv "$TMP/cedict" assets/dict/cedict.txt
echo "cedict: $(wc -l < assets/dict/cedict.txt) lines"

echo "== 2/4 ECDICT 完整版（npm tarball）=="
DL "https://registry.npmjs.org/ecdict/-/ecdict-0.0.4.tgz" "$TMP/ecdict.tgz"
tar xzf "$TMP/ecdict.tgz" -C "$TMP"
FULL="$TMP/package/assets/ecdict.csv"

echo "== 3/4 生成 mini 子集 =="
cargo run -p lexi-core --bin make_mini -- "$FULL" assets/dict/ecdict.mini.csv

echo "== 4/4 字体（若缺失）=="
mkdir -p assets/fonts
[ -s assets/fonts/JetBrainsMono-Regular.ttf ] || DL "https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/fonts/ttf/JetBrainsMono-Regular.ttf" assets/fonts/JetBrainsMono-Regular.ttf
[ -s assets/fonts/JetBrainsMono-Bold.ttf ] || DL "https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/fonts/ttf/JetBrainsMono-Bold.ttf" assets/fonts/JetBrainsMono-Bold.ttf
if [ ! -s assets/fonts/NotoSansSC-Regular.otf ] || [ ! -s assets/fonts/NotoSansSC-Bold.otf ]; then
  DL "https://github.com/notofonts/noto-cjk/releases/download/Sans2.004/18_NotoSansSC.zip" "$TMP/notosc.zip"
  unzip -o -q "$TMP/notosc.zip" -d "$TMP/notosc"
  cp "$TMP/notosc/NotoSansSC-Regular.otf" "$TMP/notosc/NotoSansSC-Bold.otf" "$TMP/notosc/LICENSE" assets/fonts/
  mv assets/fonts/LICENSE assets/fonts/OFL-NotoSansSC-LICENSE.txt
fi

echo "== 完成 =="
ls -la assets/dict assets/fonts
