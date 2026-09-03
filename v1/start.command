#!/bin/sh
cd "$(dirname "$0")"
if [ -x "node_modules/.bin/electron" ] && [ -f "node_modules/electron/path.txt" ]; then
  npm start
else
  echo "首次使用需要下载 Electron（中国网络建议使用镜像）："
  echo "npm run install:china"
  echo "如果之前下载中断，请执行：npm run repair:electron"
  echo "安装成功后请再次双击 start.command"
  read -r _
fi
