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

## 2. 先懂两个概念，再动手

**命令在哪个目录执行**：所有前端命令都在 `frontend_path` 目录里执行，后端命令都在 `backend_path` 目录里执行；
没填对应 `*_path` 时，一律在 `path`（项目根目录）执行。因此你填的命令必须是「在那个目录里能直接跑通」的命令。

**命令怎么被执行**：VibeWing 用**登录 shell** 执行——macOS / Linux 是 `$SHELL -l -c "<命令>"`，Windows 是 `cmd /d /s /c "<命令>"`。
登录 shell **不会**加载 `~/.zshrc` / `~/.bashrc`，所以 `uv`、`poetry`、nvm 的 node 这类只在你交互式终端里才有的命令很可能找不到。详见 4.3。

**三类命令不要混淆**：

| 字段 | 对应界面操作 | 含义 |
| --- | --- | --- |
| `*_cmd` | 启动 | 启动/运行服务的命令，会**常驻运行**（如 `npm run dev`、`python main.py`） |
| `*_build` | 构建生产包 | 一次性的**编译打包**命令，跑完**产出可上线产物**就退出（如 `npm run build`、`cargo build --release`） |
| `*_test_build` | 构建测试包 | 一次性的**测试**命令（如 `pytest`、`cargo test`、`vitest run`） |

**没有就不填，宁缺毋滥**：以上字段**全部可选**。该端没有可启动的东西就不填 `*_cmd`；
没有打包步骤 / 没有测试，就不填 `*_build` / `*_test_build`。留空或直接省略字段都是合法的，
VibeWing 会隐藏对应的「构建」按钮，绝不会报错或影响启动。

## 3. 字段速查表

| 字段 | 必填 | 填什么 | 什么时候留空 / 省略 |
| --- | --- | --- | --- |
| `id` | ✅ 必填 | `file:` + 文件名主干。例如文件存为 `my-app.json`，就写 `"id": "file:my-app"` | — |
| `name` | ✅ 必填 | 界面上显示的项目名，对用户友好、能与其他项目区分 | — |
| `path` | ✅ 必填 | 项目根目录的**真实绝对路径**；也是前端/后端缺省的工作目录 | — |
| `frontend_path` | 可选 | 前端源码目录的绝对路径 | 与 `path` 相同，或没有独立前端目录 |
| `frontend_cmd` | 可选 | 前端启动命令，如 `npm run dev`；若 node 是 nvm 安装的需写绝对路径 | 项目没有可启动的前端 |
| `frontend_port` | 可选 | dev server 实际监听的**固定端口**，纯数字字符串，如 `"5173"` | 端口不固定或不确定时**宁可留空也别填错** |
| `frontend_build` | 可选 | 项目 `package.json` 里**真实存在**的打包脚本，如 `npm run build` | 前端无需打包，或 `scripts` 里没有 build |
| `frontend_test_build` | 可选 | 项目里真实存在的测试命令，如 `npm run test`、`vitest run` | 没有前端测试 |
| `backend_path` | 可选 | 后端源码目录的绝对路径 | 与 `path` 相同，或没有独立后端目录 |
| `backend_cmd` | 可选 | 后端启动命令。**必须用项目内虚拟环境解释器**，如 `.venv/bin/python -m app.cli serve --env development`（详见 4.2 / 4.3） | 项目没有可启动的后端 |
| `backend_port` | 可选 | 后端实际监听的**固定端口**，纯数字字符串，如 `"8000"` | 端口动态分配或不确定时留空 |
| `backend_build` | 可选 | 后端**编译/打包**命令，如 `cargo build --release`、`go build .`、`mvn package` | **解释型语言（Python / Node / Ruby）通常没有打包步骤 → 直接省略** |
| `backend_test_build` | 可选 | 后端测试命令，如 `.venv/bin/python -m pytest`、`cargo test`、`go test ./...` | 项目没有测试 |

## 4. 最容易写错的地方（动手前必读）

### 4.1 不要把「不是构建」的命令写进 `*_build`

`*_build` 只收「跑完会产出部署产物」的命令。下面这类命令**一律不要填**，对应字段留空即可：

| ❌ 不要写进 `*_build` | 为什么 |
| --- | --- |
| `uv sync`、`pip install`、`pip install -r requirements.txt`、`poetry install`、`npm install`、`pnpm install` | 只是同步/安装依赖，不产出任何产物 |
| `python main.py`、`uvicorn app.main:app ...`、`npm run dev`、`pnpm dev`、`npm start` | 是**启动命令**，应该填在 `*_cmd`，不是构建 |
| `ruff check`、`eslint .`、`black`、`tsc --noEmit`、`mypy` | 只是代码/类型检查，不产出产物 |
| `python manage.py migrate`、`alembic upgrade head`、`prisma migrate deploy` | 数据库迁移，不是构建 |

判断标准只有一句：**这条命令跑完后，是否产出了可以上线/部署的产物？**
- `npm run build` 产出 `dist/` → 是构建 ✅
- `cargo build --release` 产出二进制 → 是构建 ✅
- `uv sync` 只是把依赖装进虚拟环境 → 不是构建 ❌，留空
- `pytest` 不产出产物，但它在跑测试 → 填 `backend_test_build`（写成 `.venv/bin/python -m pytest`，仅当项目确有测试时）

### 4.2 Python 后端怎么填（最常见的坑）

- 启动 → 填 `backend_cmd`，**用项目内的虚拟环境解释器**，例如：
  - `.venv/bin/python -m app.cli serve --env development`
  - `.venv/bin/python run.py`
  - `.venv/bin/python -m uvicorn app.main:app --host 127.0.0.1 --port 8000`
  - `.venv/bin/python manage.py runserver 0.0.0.0:8000`
- **不要写 `uv run ...` / `poetry run ...` / `pipenv run ...` / `conda activate ...`**：这些工具常常装在 `~/.local/bin`、`~/.cargo/bin`、conda 目录里，VibeWing 用登录 shell 执行时**找不到它们**（详见 4.3）。
- 项目用 uv / poetry / pip 管理依赖都无所谓，**依赖安装/同步命令不要写进任何字段**。
- 存在 `pyproject.toml` / `requirements.txt` **不代表**要打包。只有项目里确实有打包/构建产物的脚本（如发布到 PyPI 的库、PyInstaller 打包脚本）才填 `backend_build`。
- 项目有 pytest 等测试 → 填 `backend_test_build: ".venv/bin/python -m pytest"`；没有就不填。
- 结论：**绝大多数 Python Web 后端只需要** `backend_cmd` + `backend_port`，`backend_build` 和 `backend_test_build` 都省略。

### 4.3 启动命令必须真能跑起来（起不来的头号原因）

VibeWing 是用**登录 shell**执行你的命令的：macOS / Linux 上是 `$SHELL -l -c "<命令>"`，Windows 上是 `cmd /d /s /c "<命令>"`，工作目录已经切到对应的 `*_path`。由此产生两条硬性要求：

**① 不要依赖「只有交互式终端才有」的命令。** 登录 shell **不会**加载 `~/.zshrc` / `~/.bashrc`，所以装在 `~/.local/bin`、`~/.cargo/bin`、nvm 目录、conda 环境里的工具经常不在 PATH 里：

| ❌ 别这么写 | 为什么 | ✅ 改成 |
| --- | --- | --- |
| `uv run python -m app.cli serve` | `uv` 通常装在 `~/.local/bin`，登录 shell 里可能根本没有 | `.venv/bin/python -m app.cli serve` |
| `poetry run python main.py` | 同上 | `.venv/bin/python main.py` |
| `conda activate env && python main.py` | 依赖 shell 初始化，登录 shell 里常失败 | `.venv/bin/python main.py` |
| `source venv/bin/activate && python main.py` | 多余且脆弱 | `venv/bin/python main.py` |
| `cd /abs/path && python main.py` | `cd` 由 `*_path` 负责，重复写且容易写错 | 目录填进 `*_path`，命令只写 `python main.py` |
| `python main.py` / `python3 main.py` | 可能解析到系统 Python，缺项目依赖 | `.venv/bin/python main.py` |
| `uvicorn app.main:app` | uvicorn 装在项目 venv 里，不在 PATH | `.venv/bin/python -m uvicorn app.main:app` |

**② 必须带上项目真正需要的启动参数。** VibeWing 不会帮你设置环境、配置文件或环境变量，缺参数就会启动失败或用错配置：
- `--env development`（很多 CLI 用它选择 `config.<env>.yaml` 之类的配置）
- `--host 127.0.0.1 --port 8000`（**仅当项目 CLI 真的支持**才写；写死并与 `*_port` 保持一致）
- 从项目的 `README` / `AGENTS.md` / `Makefile` / 启动脚本里抄**完整**的开发启动命令，不要自己精简。

**③ 写完必须验证（强烈建议真的跑一次）。** 在对应目录里用登录 shell 试跑：
```bash
cd <后端目录>
zsh -l -c '.venv/bin/python -m app.cli serve --help'                      # 先确认入口和参数存在
timeout 10 zsh -l -c '.venv/bin/python -m app.cli serve --env development' # 真跑 10 秒，确认能起来
```
Windows 上同理用 `cmd /d /s /c "..."`。跑不起来就换命令，**绝对不要把跑不通的命令写进 JSON**。前端同理：若 node/pnpm 是 nvm 安装的，写成 `~/.nvm/versions/node/v22.x/bin/pnpm dev` 这样的绝对路径。

### 4.4 命令必须真实存在，不要自己发明

- 只从项目里**真实存在**的地方抄命令：前端抄 `package.json` 的 `scripts`；后端参考 `Makefile`、项目 README，或你自己刚验证跑通的方式。
- 不要拼凑不存在的脚本名（例如 `npm run compile:prod`），也不要“好心”帮用户补一个他认为没有的构建步骤。
- 拿不准的命令，先在对应目录里手动执行验证，能跑通再写进 JSON。

### 4.5 其他硬规则

- 所有路径必须是用户本机**真实存在的绝对路径**；禁止相对路径、占位符、`~/`（需展开成绝对路径）。
- `id` 必须等于 `file:<文件名主干>`，否则扫描后无法正确关联。
- 端口只填**固定端口**的纯数字字符串；不确定就留空——填错会让 VibeWing 的状态灯永远无法变绿。
- **不要写 `source` 字段**（VibeWing 会自动置为 `file`）。
- 只有前端或只有后端时，只填对应的那一半字段。
- 前端后端端口相同没关系，VibeWing 分别按各自的 `frontend_port` / `backend_port` 检查。

## 5. 完整示例

后端为 Python + FastAPI（无需打包）、前端为 React + Vite：

```json
{
  "id": "file:my-app",
  "name": "我的项目",
  "path": "/Users/you/code/my-app",
  "frontend_path": "/Users/you/code/my-app/web",
  "frontend_cmd": "npm run dev",
  "frontend_port": "5173",
  "frontend_build": "npm run build",
  "frontend_test_build": "npm run test",
  "backend_path": "/Users/you/code/my-app/server",
  "backend_cmd": ".venv/bin/python -m app.cli serve --env development",
  "backend_port": "8000",
  "backend_test_build": ".venv/bin/python -m pytest"
}
```

注意三点：
1. 上面**故意省略了 `backend_build`**——Python 后端不打包，就不写。
2. `backend_cmd` 用的是项目内虚拟环境解释器 `.venv/bin/python`，不是 `uv run python`，因为 `uv` 在 VibeWing 的登录 shell 里可能不存在。
3. `--env development` 是项目 CLI 真正需要的参数，缺了就会用错配置甚至起不来。**注意别自己加参数**：上面的 CLI `--help` 只支持 `-h` 和 `--env`，并没有 `--port`，所以示例里**没有**写 `--port`。端口一律以 `<命令> --help` / 源码 / README 实际支持为准，不确定的就别写，只填 `backend_port` 供 VibeWing 做健康检查。

纯后端最小示例：

```json
{
  "id": "file:my-api",
  "name": "我的 API",
  "path": "/Users/you/code/my-api",
  "backend_cmd": ".venv/bin/python run.py",
  "backend_port": "8000"
}
```

## 6. 写完后自查清单

写完后逐条核对，再交付给用户：

- [ ] 每个命令都能在它对应的工作目录里手动跑通，且真实存在于项目配置里
- [ ] 启动命令**没有**用 `uv run` / `poetry run` / `conda activate` 等依赖 PATH 的包装，而是项目内的 `.venv/bin/python`
- [ ] 启动参数完整（`--env`、`--host`、`--port` 等），并用 `zsh -l -c '<命令>'`（Windows 用 `cmd /d /s /c`）验证过能真的起来
- [ ] 没有把「依赖安装 / 启动命令 / 代码检查 / 数据库迁移」写进 `*_build` / `*_test_build`
- [ ] 无需打包、没有测试的字段已省略或留空，而不是硬塞一条命令
- [ ] `id` = `file:<文件名主干>`，且文件里没有 `source` 字段
- [ ] 所有路径都是本机真实存在的绝对路径
- [ ] 端口填的是固定端口；不确定的已留空
- [ ] 只有单端时，只填了对应半边字段

## 7. 写完之后

告诉用户：「已在 VibeWing 的项目目录生成 `<文件名>.json`，打开 VibeWing（或点界面上的『重新扫描』）就能看到，点启动即可。」

## 8. 移除

用户想移除时，删除 `projects/` 目录里对应的 `.json` 文件即可（界面里点「停止」只停进程、不删文件）。
