# VibeWing 项目导入 Skill（给 Coding Agent 用）

当你帮用户完成一个项目，或用户说「导入 VibeWing / 加到 VibeWing / 让 VibeWing 启动这个项目」时，
生成一个**项目描述 JSON** 并写入 VibeWing 的「项目目录」。VibeWing 启动（或点「重新扫描」）后会自动读取，
用户无需再手动填写 `npm run dev` / `python main.py` 之类的命令，直接一键启动。

> 这不是聊天机器人提示词，而是一段给 coding agent 的执行规范。把本文件交给你的 agent 即可。

---

## 1. 写入目录（按当前操作系统选择）

目录里每个 `*.json` 文件就是一个项目。**绝对不要**写进安装目录或 exe 旁边，而是写进下面的「数据目录」：

| 系统 | 路径 |
| --- | --- |
| macOS | `~/Library/Application Support/app.vibewing.tauri.dev/projects/` |
| Windows | `%APPDATA%\app.vibewing.tauri.dev\projects\` |
| Linux | `~/.local/share/app.vibewing.tauri.dev/projects/` |

- 不确定时，可以让用户运行 VibeWing → 主界面「打开目录」按钮复制实际路径，或直接问我（agent）去读 `get_projects_dir` 命令返回的值。
- 文件名任意，**建议用英文、无空格**，例如 `my-app.json`。文件内容的 `id` 字段写 `file:<文件名（不含扩展名）>`。
- 目录里已有一份 `README.txt` 说明格式，可读取参考。

## 2. JSON 字段

```json
{
  "id": "file:my-app",          // 必须，file:<文件名无扩展名>
  "name": "我的项目",            // 必须，界面显示名
  "path": "/abs/path/to/root",   // 必须，项目主目录（绝对路径）
  "frontend_path": "/abs/.../web",   // 可选，缺省等于 path
  "frontend_cmd": "npm run dev",     // 可选，前端启动命令
  "frontend_port": "5173",           // 可选，前端端口（填了才会有「启动中→就绪」黄灯变绿）
  "frontend_build": "npm run build", // 可选，生产构建命令
  "frontend_test_build": "",         // 可选
  "backend_path": "/abs/.../server", // 可选，缺省等于 path
  "backend_cmd": "python main.py",   // 可选，后端启动命令
  "backend_port": "8000",            // 可选，后端端口
  "backend_build": "pip install -r requirements.txt", // 可选
  "backend_test_build": ""           // 可选
}
```

## 3. 生成规则（重要）

- **所有路径必须是用户本机的真实绝对路径**，不能是你的运行环境路径、不能是相对路径、不能含占位符。
- 优先从用户实际代码库推断：前端通常在 `web` / `frontend` / `client` 目录，命令看 `package.json` 的 `scripts.dev`；后端看 `requirements.txt` / `pyproject.toml` / `Cargo.toml` / `go.mod` 推断命令与端口。
- `frontend_port` / `backend_port` 强烈建议填上（即使要猜一个常见端口），这样 VibeWing 才能显示「启动中（黄灯）→ 就绪（绿灯）」的真实状态。
- `name` 用对人类友好、能区分项目的名字。
- 不要写 `source` 字段（VibeWing 会自动置为 `file`）。
- 如果项目只有前端或只有后端，只填对应的一半即可。

## 4. 写完之后

告诉用户：「已在 VibeWing 的项目目录生成 `<文件名>.json`，打开 VibeWing（或点界面上的『重新扫描』）就能看到，点启动即可。」

## 5. 移除

用户想移除时，删除 `projects/` 目录里对应的 `.json` 文件即可（界面里点「停止」只停进程、不删文件）。
