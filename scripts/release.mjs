// 在 Gitee 创建 Release 并上传免安装单 exe。
// 供 GitHub Actions 调用（云端快网上传），本地也能跑：
//   cd src-tauri && cargo build --release
//   $env:GITEE_TOKEN="你的令牌"; node scripts/release.mjs
//
// 版本号从 src-tauri/tauri.conf.json 读取（tag = v<version>）。
// 附件取 src-tauri/target/release/aoe4-overlay.exe，上传名 AoE4Overlay-<version>.exe。
// 令牌从环境变量 GITEE_TOKEN 读取，不写入仓库。
// 上传走 Node 原生 https（fetch 对大文件有 5 分钟 header 超时，慢网会失败）。

import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import https from "node:https";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

const OWNER = "Passion4ever";
const REPO = "aoe4-overlay";
const API = `https://gitee.com/api/v5/repos/${OWNER}/${REPO}`;

const token = process.env.GITEE_TOKEN;
if (!token) {
  console.error('✗ 未设置 GITEE_TOKEN。  $env:GITEE_TOKEN="令牌"; node scripts/release.mjs');
  process.exit(1);
}

const conf = JSON.parse(await readFile(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const tag = `v${conf.version}`;

// 免安装单 exe
const exeName = `AoE4Overlay-${conf.version}.exe`; // 上传到 Gitee 的文件名
const exePath = path.join(root, "src-tauri", "target", "release", "aoe4-overlay.exe");
if (!existsSync(exePath)) {
  console.error(`✗ 找不到 ${exePath}\n  请先运行：cd src-tauri && cargo build --release`);
  process.exit(1);
}

const tokenQS = `access_token=${encodeURIComponent(token)}`;
console.log(`→ 发布 ${tag}（${exeName}）到 ${OWNER}/${REPO}`);

async function findRelease() {
  const res = await fetch(`${API}/releases?${tokenQS}&per_page=100`);
  if (!res.ok) return null;
  const list = await res.json();
  return Array.isArray(list) ? list.find((r) => r.tag_name === tag) : null;
}

// 1) 建 release，已存在则复用（幂等）
let release = null;
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
if (createRes.ok) {
  release = await createRes.json();
  console.log(`✓ release 已创建 (id=${release.id})`);
} else {
  release = await findRelease();
  if (!release) {
    console.error(`✗ 创建 release 失败 (HTTP ${createRes.status})：\n${await createRes.text()}`);
    process.exit(1);
  }
  console.log(`✓ release 已存在，复用 (id=${release.id})`);
}

// 已传过同名附件就跳过
if ((release.assets || []).some((a) => a.name === exeName)) {
  console.log(`✓ 附件 ${exeName} 已存在，跳过上传`);
  console.log(`\n🎉 https://gitee.com/${OWNER}/${REPO}/releases/tag/${tag}`);
  process.exit(0);
}

// 2) 上传安装包（原生 https，无 header 超时）
const fileData = await readFile(exePath);
const boundary = "----aoe4overlay" + Date.now();
const head = Buffer.from(
  `--${boundary}\r\n` +
    `Content-Disposition: form-data; name="file"; filename="${exeName}"\r\n` +
    `Content-Type: application/octet-stream\r\n\r\n`
);
const tail = Buffer.from(`\r\n--${boundary}--\r\n`);
const reqBody = Buffer.concat([head, fileData, tail]);
console.log(`→ 上传 ${exeName}（${(fileData.length / 1048576).toFixed(1)} MB）…`);

await new Promise((resolve, reject) => {
  const req = https.request(
    `${API}/releases/${release.id}/attach_files?${tokenQS}`,
    {
      method: "POST",
      headers: {
        "Content-Type": `multipart/form-data; boundary=${boundary}`,
        "Content-Length": reqBody.length,
      },
    },
    (res) => {
      let data = "";
      res.on("data", (c) => (data += c));
      res.on("end", () =>
        res.statusCode >= 200 && res.statusCode < 300
          ? resolve(data)
          : reject(new Error(`HTTP ${res.statusCode}: ${data}`))
      );
    }
  );
  req.on("error", reject);
  req.write(reqBody);
  req.end();
});

console.log(`✓ 已上传 ${exeName}`);
console.log(`\n🎉 https://gitee.com/${OWNER}/${REPO}/releases/tag/${tag}`);
