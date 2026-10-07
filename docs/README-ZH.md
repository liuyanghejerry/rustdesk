<p align="center">
  <img src="../res/logo-header.svg" alt="RustDesk - Your remote desktop"><br>
  <a href="#免费的公共服务器">服务器</a> •
  <a href="#基本构建步骤">编译</a> •
  <a href="#使用-Docker-编译">Docker</a> •
  <a href="#文件结构">结构</a> •
  <a href="#截图">截图</a><br>
  [<a href="../README.md">English</a>] | [<a href="README-UA.md">Українська</a>] | [<a href="README-CS.md">česky</a>] | [<a href="README-HU.md">Magyar</a>] | [<a href="README-ES.md">Español</a>] | [<a href="README-FA.md">فارسی</a>] | [<a href="README-FR.md">Français</a>] | [<a href="README-DE.md">Deutsch</a>] | [<a href="README-PL.md">Polski</a>] | [<a href="README-ID.md">Indonesian</a>] | [<a href="README-FI.md">Suomi</a>] | [<a href="README-ML.md">മലയാളം</a>] | [<a href="README-JP.md">日本語</a>] | [<a href="README-NL.md">Nederlands</a>] | [<a href="README-IT.md">Italiano</a>] | [<a href="README-RU.md">Русский</a>] | [<a href="README-PTBR.md">Português (Brasil)</a>] | [<a href="README-EO.md">Esperanto</a>] | [<a href="README-KR.md">한국어</a>] | [<a href="README-AR.md">العربي</a>] | [<a href="README-VN.md">Tiếng Việt</a>] | [<a href="README-GR.md">Ελληνικά</a>]<br>
</p>

## 本 fork 的区别与演示

这是 [RustDesk](https://github.com/rustdesk/rustdesk) 的实验性 fork，主要改善 Android 控制 Linux 时的终端体验，便于使用远程命令行和已安装的 CLI 编程工具，并非官方发行版。

### 实际演示

视频和截图来自 Android 模拟器与隔离的 Linux 虚拟机之间的真实连接，不依赖真机。视频中的示例命令、CI 输出及图表均为专门生成的演示内容；延迟、接收速率和桌面 FPS 是连接的实时读数。下方新增编程截图使用真实模型调用和独立样例项目，认证过程不进入截图，没有使用个人项目文件。

| 终端、快捷键分组和图片放大 | 保留 shell 后续接 |
| --- | --- |
| <img src="assets/mobile-terminal/workflow.gif" width="300" alt="展开快捷键分组、执行示例命令、预览并放大终端中的 PNG 图片"> | <img src="assets/mobile-terminal/shell-resume.gif" width="300" alt="选择保留并退出，重新打开终端，检查相同的 shell PID 和环境变量"> |
| [观看 MP4](assets/mobile-terminal/workflow.mp4) | [观看 MP4](assets/mobile-terminal/shell-resume.mp4) |

[网络中断与重连视频](assets/mobile-terminal/network-reconnect.mp4) · [截图、录制说明与复现步骤](mobile-terminal-demo.md)

#### Codex / Kimi Code 实际编程

两款工具都在远程 Linux 虚拟机中实际生成了 `greet.py` 和三个 Python 单元测试，并运行通过。这些是 Android 终端里看到的真实 CLI 画面，没有伪造模型回复。Codex CLI 0.160.0 使用 GPT-6.1-Sol；Kimi Code CLI 2.1.1 沿用本地配置的 GLM-5.3，并非 Kimi 模型。CLI 工具和账号需要用户另外安装配置。

| Codex：生成代码与测试通过 | Kimi Code：生成代码与测试通过 |
| --- | --- |
| [<img src="assets/mobile-terminal/codex-result.png" width="300" alt="Codex 实际生成的 Python 测试 diff 和三个通过的测试">](assets/mobile-terminal/codex-result.png) | [<img src="assets/mobile-terminal/kimi-code-result.png" width="300" alt="Kimi Code 实际生成的 Python 代码和测试通过结果">](assets/mobile-terminal/kimi-code-result.png) |

[提示词与截图说明](mobile-terminal-demo.md#actual-codex-and-kimi-code-sessions--真实编程会话)

### 与官方基线的区别

对比以本分支的 upstream 基线 [`c9c0b5d0`](https://github.com/rustdesk/rustdesk/commit/c9c0b5d0efd44b364b31639f3c118e8adc42d98c) 为准，不代表所有官方版本。该基线已经有独立的 **Terminal (beta)** 连接和终端会话功能；我们新增的是桌面连接内的终端路径。

| 功能 | 官方基线 | 本 fork |
| --- | --- | --- |
| 终端入口 | 独立的 Terminal (beta) 连接 | 在 Android 已认证的桌面连接内打开 Terminal，复用连接 |
| 手机操作 | 既有独立终端界面 | 控制、光标、输入与编辑三组可展开按钮，包含完整方向键、Home/End、Ctrl+C/D/Z、Tab 与 Enter |
| 图片输出 | 既有终端渲染 | 点击输出中的图片路径，或手动输入路径，从被控端读取图片并预览、双击或双指放大 |
| 网络反馈 | 既有连接质量工具 | 终端状态栏显示 RTT、总接收速率、桌面 FPS，以及延迟回复过期状态 |
| 主机资源 | 既有主机工具 | 终端顶部显示远程 Linux 的 RAM 和根文件系统 `/` 使用量，每五秒刷新 |
| 桌面视频 | 既有桌面行为 | 进入终端时暂停当前连接的桌面视频订阅，返回桌面时恢复 |
| shell 生命周期 | 既有独立会话行为 | 网络断开保留当前 shell；主动退出可取消、保留或销毁 |
| 输入传输 | 既有终端输入路径 | 新通道按顺序分块、等待写入确认；失败后停止剩余输入；进入终端自动启用输入法 |

既有独立终端路径保留。CLI 编程代理在远程机器上自行安装运行，本 fork 没有内置 AI 模型、代理或 API 密钥。

### 安装与使用

主控 Android 和被控 Linux 都需要兼容的 fork 版本，下方官方版下载链接不包含这些改动。被控端需要运行登录用户的服务进程并开启终端权限；新终端拒绝提供 root shell。连接桌面后点击 **⋮ → Terminal** 即可使用。不需要更换 `hbbs`/`hbbr` 服务端；当前验证范围为 Android → Linux。

网络断开可续接，但被控端 RustDesk 进程重启后不能保留 shell。每个保留 shell 只缓存最近 1 MiB 输出，超出会提示，最多保留 100 个。重连仍需认证和权限，断线期间未确认的输入不会自动重发。接收速率包含整个连接，模拟器的低 RTT 不代表公网表现。

资源状态显示的是被控 Linux 的使用百分比和已用/总量 GiB。RAM 使用量不含内核标记为可用的内存；磁盘统计根文件系统 `/` 的已分配空间，不包括所有挂载盘，也不跟随 shell 当前目录。读取失败、断网或过期时显示“—”。这项读数需要同时更新主控端与被控端，旧版被控端仍可使用终端，但没有资源读数。

已知问题：本地构建曾被 Vivo 判定为“隐私信息窃取”，专用签名的 Release 版仍有提示，原因尚未确定。本 fork 尚未获得 Vivo 或应用商店认证。

下方保留的是 upstream 项目信息与下载入口。


> [!CAUTION]
> **免责声明:** <br>
> RustDesk 的开发人员不纵容或支持任何不道德或非法的软件使用行为。滥用行为，例如未经授权的访问、控制或侵犯隐私，严格违反我们的准则。作者对应用程序的任何滥用行为概不负责。

与我们交流: [知乎](https://www.zhihu.com/people/rustdesk) | [Discord](https://discord.gg/nDceKgxnkV) | [Reddit](https://www.reddit.com/r/rustdesk) | [YouTube](https://www.youtube.com/@rustdesk)

[![RustDesk Server Pro](https://img.shields.io/badge/RustDesk%20Server%20Pro-%E9%AB%98%E7%BA%A7%E5%8A%9F%E8%83%BD-blue)](https://rustdesk.com/pricing.html)

远程桌面软件，开箱即用，无需任何配置。您完全掌控数据，不用担心安全问题。您可以使用我们的注册/中继服务器，
或者[自己设置](https://rustdesk.com/server)，
亦或者[开发您的版本](https://github.com/rustdesk/rustdesk-server-demo)。

![image](https://user-images.githubusercontent.com/71636191/171661982-430285f0-2e12-4b1d-9957-4a58e375304d.png)

RustDesk 期待各位的贡献. 如何参与开发? 详情请看 [CONTRIBUTING-ZH.md](CONTRIBUTING-ZH.md).

[**FAQ**](https://github.com/rustdesk/rustdesk/wiki/FAQ)

[**BINARY DOWNLOAD**](https://github.com/rustdesk/rustdesk/releases)

[**NIGHTLY BUILD**](https://github.com/rustdesk/rustdesk/releases/tag/nightly)

[<img src="https://fdroid.gitlab.io/artwork/badge/get-it-on.png"
    alt="Get it on F-Droid"
    height="80">](https://f-droid.org/en/packages/com.carriez.flutter_hbb)

## 依赖

桌面版本使用 Flutter 或 Sciter（已弃用）作为 GUI，本教程仅适用于 Sciter，因为它更简单且更易于上手。查看我们的[CI](https://github.com/rustdesk/rustdesk/blob/master/.github/workflows/flutter-build.yml)以构建 Flutter 版本。

请自行下载Sciter动态库。

[Windows](https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.win/x64/sciter.dll) |
[Linux](https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.lnx/x64/libsciter-gtk.so) |
[macOS](https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.osx/libsciter.dylib)

## 基本构建步骤

- 请准备好 Rust 开发环境和 C++ 编译环境

- 安装 [vcpkg](https://github.com/microsoft/vcpkg), 正确设置 `VCPKG_ROOT` 环境变量

  - Windows: vcpkg install libvpx:x64-windows-static libyuv:x64-windows-static opus:x64-windows-static aom:x64-windows-static
  - Linux/macOS: vcpkg install libvpx libyuv opus aom

- 运行 `cargo run`

## [构建](https://rustdesk.com/docs/en/dev/build/)

## 在 Linux 上编译

### Ubuntu 18 (Debian 10)

```sh
sudo apt install -y zip g++ gcc git curl wget nasm yasm libgtk-3-dev clang libxcb-randr0-dev libxdo-dev \
        libxfixes-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libpulse-dev cmake make \
        libclang-dev ninja-build libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
```

### openSUSE Tumbleweed 

```sh
sudo zypper install gcc-c++ git curl wget nasm yasm gcc gtk3-devel clang libxcb-devel libXfixes-devel cmake alsa-lib-devel gstreamer-devel gstreamer-plugins-base-devel xdotool-devel
```

### Fedora 28 (CentOS 8)

```sh
sudo yum -y install gcc-c++ git curl wget nasm yasm gcc gtk3-devel clang libxcb-devel libxdo-devel libXfixes-devel pulseaudio-libs-devel cmake alsa-lib-devel
```

### Arch (Manjaro)

```sh
sudo pacman -Syu --needed unzip git cmake gcc curl wget yasm nasm zip make pkg-config clang gtk3 xdotool libxcb libxfixes alsa-lib pipewire
```

### 安装 vcpkg

```sh
git clone https://github.com/microsoft/vcpkg
cd vcpkg
git checkout 2023.04.15
cd ..
vcpkg/bootstrap-vcpkg.sh
export VCPKG_ROOT=$HOME/vcpkg
vcpkg/vcpkg install libvpx libyuv opus aom
```

### 修复 libvpx (仅仅针对 Fedora)

```sh
cd vcpkg/buildtrees/libvpx/src
cd *
./configure
sed -i 's/CFLAGS+=-I/CFLAGS+=-fPIC -I/g' Makefile
sed -i 's/CXXFLAGS+=-I/CXXFLAGS+=-fPIC -I/g' Makefile
make
cp libvpx.a $HOME/vcpkg/installed/x64-linux/lib/
cd
```

### 构建

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
git clone https://github.com/rustdesk/rustdesk
cd rustdesk
mkdir -p target/debug
wget https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.lnx/x64/libsciter-gtk.so
mv libsciter-gtk.so target/debug
VCPKG_ROOT=$HOME/vcpkg cargo run
```

## 使用 Docker 编译

克隆版本库并构建 Docker 容器:

```sh
git clone https://github.com/rustdesk/rustdesk # 克隆Github存储库
cd rustdesk # 进入文件夹
docker build -t "rustdesk-builder" . # 构建容器
```

请注意：
* 针对国内网络访问问题，可以做以下几点优化：  
   1. Dockerfile 中修改系统的源到国内镜像
      ```
      在Dockerfile的RUN apt update之前插入两行：
   
      RUN sed -i "s|deb.debian.org|mirrors.aliyun.com|g" /etc/apt/sources.list && \
          sed -i "s|security.debian.org|mirrors.aliyun.com|g" /etc/apt/sources.list
      ```

   2. 修改容器系统中的 cargo 源，在`RUN ./rustup.sh -y`后插入下面代码：

      ```
      RUN echo '[source.crates-io]' > ~/.cargo/config \
       && echo 'registry = "https://github.com/rust-lang/crates.io-index"'  >> ~/.cargo/config \
       && echo '# 替换成你偏好的镜像源'  >> ~/.cargo/config \
       && echo "replace-with = 'sjtu'"  >> ~/.cargo/config \
       && echo '# 上海交通大学'   >> ~/.cargo/config \
       && echo '[source.sjtu]'   >> ~/.cargo/config \
       && echo 'registry = "https://mirrors.sjtug.sjtu.edu.cn/git/crates.io-index"'  >> ~/.cargo/config \
       && echo '' >> ~/.cargo/config
      ```

   3. Dockerfile 中加入代理的 env

      ```
      在User root后插入两行

      ENV http_proxy=http://host:port
      ENV https_proxy=http://host:port
      ```

   4. docker build 命令后面加上 proxy 参数

      ```
      docker build -t "rustdesk-builder" . --build-arg http_proxy=http://host:port --build-arg https_proxy=http://host:port
      ```

### 构建 RustDesk 程序

然后, 每次需要构建应用程序时, 运行以下命令:

```sh
docker run --rm -it -v $PWD:/home/user/rustdesk -v rustdesk-git-cache:/home/user/.cargo/git -v rustdesk-registry-cache:/home/user/.cargo/registry -e PUID="$(id -u)" -e PGID="$(id -g)" rustdesk-builder
```

请注意:  
* 因为需要缓存依赖项，首次构建一般很慢（国内网络会经常出现拉取失败，可以多试几次）。  
* 如果您需要添加不同的构建参数，可以在指令末尾的`<OPTIONAL-ARGS>` 位置进行修改。例如构建一个"Release"版本，在指令后面加上` --release`即可。
* 如果出现以下的提示，则是无权限问题，可以尝试把`-e PUID="$(id -u)" -e PGID="$(id -g)"`参数去掉。
   ```
   usermod: user user is currently used by process 1
   groupmod: Permission denied.
   groupmod: cannot lock /etc/group; try again later.
   ```
   > **原因：** 容器的 entrypoint 脚本会检测 UID 和 GID，在度判和给定的环境变量的不一致时，会强行修改 user 的 UID 和 GID 并重新运行。但在重启后读不到环境中的 UID 和 GID，然后再次进入判错重启环节


### 运行 RustDesk 程序

生成的可执行程序在 target 目录下，可直接通过指令运行调试 (Debug) 版本的 RustDesk:
```sh
target/debug/rustdesk
```

或者您想运行发行 (Release) 版本:

```sh
target/release/rustdesk
```

请注意：
* 请保证您运行的目录是在 RustDesk 库的根目录内，否则软件会读不到文件。
* `install`、`run`等 Cargo 的子指令在容器内不可用，宿主机才行。

## 文件结构

- **[libs/hbb_common](https://github.com/rustdesk/rustdesk/tree/master/libs/hbb_common)**: 视频编解码, 配置, tcp/udp 封装, protobuf, 文件传输相关文件系统操作函数, 以及一些其他实用函数
- **[libs/scrap](https://github.com/rustdesk/rustdesk/tree/master/libs/scrap)**: 屏幕截取
- **[libs/enigo](https://github.com/rustdesk/rustdesk/tree/master/libs/enigo)**: 平台相关的鼠标键盘输入
- **[libs/clipboard](https://github.com/rustdesk/rustdesk/tree/master/libs/clipboard)**: Windows、Linux、macOS 的文件复制和粘贴实现
- **[src/ui](https://github.com/rustdesk/rustdesk/tree/master/src/ui)**: 过时的 Sciter UI（已弃用）
- **[src/server](https://github.com/rustdesk/rustdesk/tree/master/src/server)**: 被控端服务音频、剪切板、输入、视频服务、网络连接的实现
- **[src/client.rs](https://github.com/rustdesk/rustdesk/tree/master/src/client.rs)**: 控制端
- **[src/rendezvous_mediator.rs](https://github.com/rustdesk/rustdesk/tree/master/src/rendezvous_mediator.rs)**: 与[rustdesk-server](https://github.com/rustdesk/rustdesk-server)保持UDP通讯, 等待远程连接（通过打洞直连或者中继）
- **[src/platform](https://github.com/rustdesk/rustdesk/tree/master/src/platform)**: 平台服务相关代码
- **[flutter](https://github.com/rustdesk/rustdesk/tree/master/flutter)**: 适用于桌面和移动设备的 Flutter 代码

## 截图

![image](https://user-images.githubusercontent.com/71636191/113112362-ae4deb80-923b-11eb-957d-ff88daad4f06.png)

![image](https://user-images.githubusercontent.com/71636191/113112619-f705a480-923b-11eb-911d-97e984ef52b6.png)

![image](https://user-images.githubusercontent.com/71636191/113112857-3fbd5d80-923c-11eb-9836-768325faf906.png)

![image](https://user-images.githubusercontent.com/71636191/135385039-38fdbd72-379a-422d-b97f-33df71fb1cec.png)
