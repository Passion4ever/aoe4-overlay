// 在 Gitee 创建 Release 并上传打包好的 exe。
//
// 用法（PowerShell）：
//   npm run build
//   $env:GITEE_TOKEN="你的私人令牌"; npm run release
//
// 版本号从 package.json 读取，附件取 dist/AoE4Overlay-<version>.exe。
// 令牌从环境变量 GITEE_TOKEN 读取，不写入仓库。

import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

const OWNER = "Passion4ever";
const REPO = "aoe4-overlay";
const API = `https://gitee.com/api/v5/repos/${OWNER}/${REPO}`;

const token = process.env.GITEE_TOKEN;
if (!token) {
  console.error("✗ 未设置 GITEE_TOKEN 环境变量。");
  console.error('  PowerShell:  $env:GITEE_TOKEN="你的令牌"; npm run release');
  process.exit(1);
}

const pkg = JSON.parse(await readFile(path.join(root, "package.json"), "utf8"));
const version = pkg.version;
const tag = `v${version}`;
const exeName = `AoE4Overlay-${version}.exe`;
const exePath = path.join(root, "dist", exeName);

if (!existsSync(exePath)) {
  console.error(`✗ 找不到 ${exePath}\n  请先运行：npm run build`);
  process.exit(1);
}

const tokenQS = `access_token=${encodeURIComponent(token)}`;
console.log(`→ 发布 ${tag}（${exeName}）到 ${OWNER}/${REPO}`);

// 1) 创建 release（若 tag 不存在，会基于 target_commitish 自动创建该 tag）
const createRes = await fetch(`${API}/releases?${tokenQS}`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    tag_name: tag,
    name: tag,
    body: `AoE4 Overlay ${tag}`,
    target_commitish: "main",
  }),
});
const createText = await createRes.text();
if (!createRes.ok) {
  console.error(`✗ 创建 release 失败 (HTTP ${createRes.status})：\n${createText}`);
  console.error("  若提示 tag 已存在，请先在 Gitee 删掉同名 release/tag 再重试。");
  process.exit(1);
}
const release = JSON.parse(createText);
console.log(`✓ release 已创建 (id=${release.id})`);

// 2) 上传 exe 附件
const buf = await readFile(exePath);
const form = new FormData();
form.append("file", new Blob([buf]), exeName);
const upRes = await fetch(`${API}/releases/${release.id}/attach_files?${tokenQS}`, {
  method: "POST",
  body: form,
});
const upText = await upRes.text();
if (!upRes.ok) {
  console.error(`✗ 上传附件失败 (HTTP ${upRes.status})：\n${upText}`);
  process.exit(1);
}
console.log(`✓ 已上传 ${exeName}（${(buf.length / 1048576).toFixed(1)} MB）`);
console.log(`\n🎉 完成：https://gitee.com/${OWNER}/${REPO}/releases/tag/${tag}`);
