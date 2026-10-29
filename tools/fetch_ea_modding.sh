#!/usr/bin/env bash
# 重建 data/ea-modding/red-alert-3：从 EA 官方 CnC_Modding_Support 仓库取 RA3 的
# Xml / Schemas / Libraries / Shaders，跳过 132 MB 的 .tga 纹理。
#
# 用法：
#   tools/fetch_ea_modding.sh              # 默认写到 data/ea-modding
#   tools/fetch_ea_modding.sh /tmp/mirror  # 换目标目录
#
# 可用环境变量：
#   CNC_MODDING_SUPPORT_URL  仓库地址，默认 https://github.com/electronicarts/CnC_Modding_Support
#   CNC_MODDING_SUPPORT_REV  分支或 tag，默认 main
#   CNC_MODDING_SUPPORT_PROXY  git 走代理时填写，例如 http://127.0.0.1:10808
#
# 说明：用 --filter=blob:none --depth 1 的部分克隆 + 稀疏检出，只下载选中的文件；
# 结果不写入版本库（见 .gitignore 的 /data/）。清单 data/ea-modding/manifest.jsonl
# 记录每个文件的字节数与 sha256，便于核对上游是否变化。

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

REPO_URL="${CNC_MODDING_SUPPORT_URL:-https://github.com/electronicarts/CnC_Modding_Support}"
REV="${CNC_MODDING_SUPPORT_REV:-main}"
DEST="${1:-data/ea-modding}"
TARGET="$DEST/red-alert-3"

git_cmd=(git)
if [ -n "${CNC_MODDING_SUPPORT_PROXY:-}" ]; then
    git_cmd+=(-c "http.proxy=$CNC_MODDING_SUPPORT_PROXY" -c "https.proxy=$CNC_MODDING_SUPPORT_PROXY")
fi

sparse_patterns=(
    "/Red Alert 3/Xml/"
    "/Red Alert 3/Schemas/"
    "/Red Alert 3/Libraries/"
    "/Red Alert 3/Shaders/"
    "/Red Alert 3/RA3Music.h"
    "!/Red Alert 3/Xml/**/*.tga"
)

scratch=$(mktemp -d "${TMPDIR:-/tmp}/ea-modding.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

echo "==> clone $REPO_URL ($REV)"
"${git_cmd[@]}" clone --filter=blob:none --no-checkout --depth 1 \
    --branch "$REV" "$REPO_URL" "$scratch/repo"

echo "==> sparse checkout"
cd "$scratch/repo"
"${git_cmd[@]}" sparse-checkout init --no-cone
"${git_cmd[@]}" sparse-checkout set "${sparse_patterns[@]}"
"${git_cmd[@]}" checkout "$REV"

commit=$("${git_cmd[@]}" rev-parse HEAD)
commit_date=$("${git_cmd[@]}" log -1 --format=%cI)
cd "$repo_root"

echo "==> replace $TARGET"
mkdir -p "$TARGET"
rm -rf "${TARGET:?}/Xml" "${TARGET:?}/Schemas" "${TARGET:?}/Libraries" \
       "${TARGET:?}/Shaders" "${TARGET:?}/RA3Music.h"
for entry in Xml Schemas Libraries Shaders RA3Music.h; do
    cp -R "$scratch/repo/Red Alert 3/$entry" "$TARGET/$entry"
done

if command -v sha256sum >/dev/null 2>&1; then
    sha_tool=(sha256sum)
else
    sha_tool=(shasum -a 256)
fi

echo "==> manifest + provenance"
manifest="$DEST/manifest.jsonl"
: >"$manifest"
count=0
total=0
while IFS= read -r rel; do
    rel=${rel#./}
    file="$TARGET/$rel"
    bytes=$(wc -c <"$file" | tr -d ' ')
    sum=$("${sha_tool[@]}" -- "$file")
    sum=${sum%% *}
    printf '{"bytes":%s,"path":"%s","sha256":"%s"}\n' "$bytes" "$rel" "$sum" >>"$manifest"
    count=$((count + 1))
    total=$((total + bytes))
done < <(cd "$TARGET" && find . -type f | LC_ALL=C sort)

cat >"$DEST/provenance.json" <<JSON
{
  "source": "$REPO_URL",
  "subtree": "Red Alert 3",
  "revision": "$commit",
  "revision_date": "$commit_date",
  "fetched_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "files": $count,
  "bytes": $total,
  "excluded": ["Red Alert 3/Xml/**/*.tga", "上游其它游戏目录"],
  "license": "GPL-3.0 with additional terms (upstream LICENSE.md)"
}
JSON

echo "done: $count files, $((total / 1048576)) MB in $TARGET"
echo "revision: $commit ($commit_date)"
