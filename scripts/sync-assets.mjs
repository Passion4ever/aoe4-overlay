// 半自动维护素材 / 文明数据：跟踪上游 aoe4world/overlay
//
// 做三件事：
//   1. 同步 flags / badges 图片：对比上游目录，缺啥下啥（文件名与上游一字不差）
//   2. 检测新文明：上游 query.ts 的 CIVILIZATIONS 表里有、而本地 data.js 没有的 → 报警
//   3. 半自动补 data.js：把新文明的 color / flag 自动填好写进 data.js，cn 中文名留空等你补
//
// 用法：
//   node scripts/sync-assets.mjs          下载缺失图片 + 把新文明写入 data.js（cn 留空）
//   node scripts/sync-assets.mjs --dry    只报告差异，不下载、不改任何文件
//
// 中文名(cn) 和地图翻译(MAP_CN) 是翻译活，机器填不了，永远得人工。本脚本只把这一步缩到最小。

import { readFile, writeFile, access, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_JS = join(ROOT, "src", "data.js");
const FLAGS_DIR = join(ROOT, "src", "assets", "flags");
const BADGES_DIR = join(ROOT, "src", "assets", "badges");

const REPO = "aoe4world/overlay";
const BRANCH = "main";
const RAW = (p) => `https://raw.githubusercontent.com/${REPO}/${BRANCH}/${p}`;
const API = (p) => `https://api.github.com/repos/${REPO}/contents/${p}?ref=${BRANCH}`;

const DRY = process.argv.includes("--dry");
const log = (...a) => console.log(...a);

async function exists(p) {
  try { await access(p); return true; } catch { return false; }
}

async function fetchText(url) {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${r.status} ${url}`);
  return r.text();
}

// 列上游目录文件名；失败(限流/网络)返回 null，调用方自行降级
async function listDir(path) {
  try {
    const r = await fetch(API(path), { headers: { "User-Agent": "aoe4-overlay-sync" } });
    if (!r.ok) return null;
    const j = await r.json();
    if (!Array.isArray(j)) return null;
    return j.filter((e) => e.type === "file").map((e) => e.name);
  } catch { return null; }
}

async function download(remotePath, destPath) {
  const r = await fetch(RAW(remotePath));
  if (!r.ok) throw new Error(`下载失败 ${r.status}: ${remotePath}`);
  const buf = Buffer.from(await r.arrayBuffer());
  await writeFile(destPath, buf);
}

// ---- 1. 解析上游文明表 query.ts -> { slug: {color, flag} } ----
async function fetchUpstreamCivs() {
  const src = await fetchText(RAW("src/overlay/query.ts"));
  const block = src.match(/CIVILIZATIONS[^=]*=\s*\{([\s\S]*?)\n\};/);
  if (!block) throw new Error("没在上游 query.ts 里找到 CIVILIZATIONS 表，上游结构可能变了");
  const civs = {};
  // 形如:  abbasid_dynasty: { name: "...", color: "#5D6063", flag: FLAGS.abbasid },
  const re = /(\w+):\s*\{[^}]*?color:\s*"(#[0-9A-Fa-f]+)"[^}]*?flag:\s*FLAGS\.(\w+)/g;
  let m;
  while ((m = re.exec(block[1]))) civs[m[1]] = { color: m[2], flag: m[3] };
  return civs;
}

// ---- 2. 读本地 data.js 的 CIVS ----
async function readLocalData() {
  const text = await readFile(DATA_JS, "utf8");
  const m = text.match(/window\.CIVS=(\{[\s\S]*?\});/);
  if (!m) throw new Error("data.js 里没找到 window.CIVS={...};");
  return { text, civs: JSON.parse(m[1]) };
}

// ---- 图片同步 ----
async function syncImages(label, dirPath, localExt, remoteDir, fallbackNames) {
  if (!DRY) await mkdir(dirPath, { recursive: true });
  let names = await listDir(remoteDir);
  if (!names) {
    log(`  ⚠ 无法列出上游 ${label} 目录(可能 GitHub API 限流)，降级为只补已知缺失项`);
    names = fallbackNames;
  }
  names = names.filter((n) => n.endsWith(localExt));
  let missing = 0;
  for (const name of names) {
    const dest = join(dirPath, name);
    if (await exists(dest)) continue;
    missing++;
    log(`  ↓ ${DRY ? "缺(将下载)" : "下载"} ${label}/${name}`);
    if (!DRY) await download(`${remoteDir}/${name}`, dest);
  }
  if (missing === 0) log(`  ✓ ${label} 已是最新`);
}

async function main() {
  log(`同步上游 ${REPO}@${BRANCH}${DRY ? "  [dry-run 只报告]" : ""}\n`);

  // 文明差异
  log("【文明】");
  const upstream = await fetchUpstreamCivs();
  const { text, civs: local } = await readLocalData();
  const newSlugs = Object.keys(upstream).filter((s) => !(s in local));
  const goneSlugs = Object.keys(local).filter((s) => !(s in upstream));

  if (goneSlugs.length) log(`  ℹ 本地有、上游已无（一般不用动）: ${goneSlugs.join(", ")}`);

  if (!newSlugs.length) {
    log("  ✓ 没有新文明，data.js 文明表已覆盖上游全部\n");
  } else {
    log(`  ⚠ 发现 ${newSlugs.length} 个新文明：`);
    const merged = { ...local };
    for (const slug of newSlugs) {
      const u = upstream[slug];
      merged[slug] = { color: u.color, flag: u.flag, cn: "" }; // cn 留空等人工
      log(`     ${slug}  (flag=${u.flag}, color=${u.color})  → cn 待填`);
    }
    if (!DRY) {
      // 只替换 window.CIVS=...; 这一段，MAP_CN 与注释原样保留
      const compact = "window.CIVS=" + JSON.stringify(merged) + ";";
      await writeFile(DATA_JS, text.replace(/window\.CIVS=\{[\s\S]*?\};/, compact), "utf8");
      log(`  ✏ 已写入 data.js（color/flag 已填好）。请补这些文明的 cn 中文名后再发版：`);
      log(`     ${newSlugs.join("、")}`);
    }
    log("");
  }

  // 图片同步（flags 的兜底名取自上游文明表的 flag 字段 + unknown）
  log("【flags 图片】");
  const flagFallback = [...new Set(Object.values(upstream).map((c) => c.flag + ".png")), "unknown.png"];
  await syncImages("flags", FLAGS_DIR, ".png", "src/assets/flags", flagFallback);

  // 兜底：solo/team × 段位×档位 + unranked
  const tiers = ["bronze", "silver", "gold", "platinum", "diamond", "conqueror"];
  const badgeStems = [];
  for (const side of ["solo", "team"]) {
    for (const t of tiers) for (const n of [1, 2, 3]) badgeStems.push(`${side}_${t}_${n}`);
    badgeStems.push(`${side}_unranked`);
  }

  log("【badges 默认风格(svg)】");
  await syncImages("badges/default", join(BADGES_DIR, "default"), ".svg",
    "src/assets/badges/s3", badgeStems.map((s) => s + ".svg"));

  log("【badges 游戏原版(png)】");
  await syncImages("badges/ingame", join(BADGES_DIR, "ingame"), ".png",
    "src/assets/badges/s3_ingame", badgeStems.map((s) => s + ".png"));

  log("\n完成。" + (DRY ? "（dry-run，未改动任何文件）" : ""));
  if (newSlugs.length && !DRY)
    log(`提醒：data.js 里有 ${newSlugs.length} 个文明的 cn 还是空字符串，补上中文名再发版。`);
}

main().catch((e) => { console.error("出错：", e.message); process.exit(1); });
