import argparse
from datetime import datetime
import hashlib
import os
from pathlib import Path
import plistlib
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import zipfile

APP_NAME = '轻笺.app'
ARCHIVE_NAME = '轻笺-苹果桌面.zip'
NOTES_NAME = '首次打开说明.txt'
IDENTIFIER = 'com.qingjian.notebook'
OPENING_GUIDE = '''轻笺：首次打开说明

一、安装与首次打开

请先解压整个压缩包，将“轻笺”拖入“应用程序”文件夹，再双击打开。
如果系统提示无法验证开发者或无法检查是否包含恶意软件，请确认文件来自可信任的发送者，关闭提示，然后：

1. 打开“系统设置”。
2. 进入“隐私与安全性”，向下滚动。
3. 找到刚刚被阻止的“轻笺”，点击“仍要打开”。
4. 按系统要求验证身份，再点击“打开”。

请先尝试打开应用，系统设置中才可能出现对应按钮。系统会保存该应用的例外，之后通常可以直接双击打开；更新版本后可能需要重新确认。
较新的系统不能仅靠右键打开来完成这一步，请使用上述系统设置入口。受单位管理的设备可能需要管理员允许。
整个操作不需要打开终端，也不需要关闭系统的整体安全保护。

二、这个安装包的签名状态

本包使用免费的临时签名，打包时已执行签名完整性检查，并核对压缩后的文件内容、中文路径、符号链接及执行权限。
临时签名不能证明开发者身份，也不等于苹果公证，因此首次打开时仍可能出现系统提示。这不是苹果对应用安全性的认证。

如果出现“应用已损坏”“将损坏你的电脑”等提示，或没有“仍要打开”入口，请停止尝试并联系发送者，提供完整提示截图和系统版本，以便检查文件完整性及系统限制。

三、日常使用与升级

笔记保存在本机工作区，不在应用本体中。升级前建议在设置中导出轻笺备份；退出旧版后再替换应用。不要删除工作区数据目录。
本包仅适用于苹果桌面系统，不能作为手机安装包使用。若系统提示处理器不兼容，请向发送者索取对应机型版本。

四、官方帮助

苹果官方首次打开指引：
https://support.apple.com/zh-cn/102445

苹果关于新版系统打开方式的说明：
https://developer.apple.com/news/?id=saqachfa
'''


def inventory(app):
    result = {}
    base = app.resolve()
    for path in sorted(app.rglob('*')):
        if path.is_symlink():
            if not path.resolve().is_relative_to(base):
                raise ValueError('应用包含指向包外的符号链接')
            data = os.readlink(path).encode('utf-8')
        elif path.is_file():
            data = path.read_bytes()
        elif path.is_dir():
            continue
        else:
            raise ValueError('应用包含不支持的文件类型')
        result[path.relative_to(app).as_posix()] = (path.lstat().st_mode, data)
    return result


def validate_app(app):
    if app.is_symlink() or not app.is_dir():
        raise ValueError('未找到可分发的应用目录')
    with (app / 'Contents/Info.plist').open('rb') as file:
        info = plistlib.load(file)
    executable = info.get('CFBundleExecutable', '')
    if not executable or Path(executable).name != executable or info.get('CFBundleIdentifier') != IDENTIFIER:
        raise ValueError('应用标识或可执行文件配置无效')
    binary = app / 'Contents/MacOS' / executable
    if not binary.is_file() or not binary.stat().st_mode & 0o111:
        raise ValueError('应用可执行文件缺失或没有执行权限')
    inventory(app)


def payload(app, notes):
    result = {APP_NAME + '/' + name: entry for name, entry in inventory(app).items()}
    result[NOTES_NAME] = (notes.stat().st_mode, notes.read_bytes())
    return result


def write_archive(archive, app, notes):
    with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as output:
        for name, (mode, data) in payload(app, notes).items():
            item = zipfile.ZipInfo(name, datetime.now().timetuple()[:6])
            item.create_system = 3
            item.external_attr = mode << 16
            item.compress_type = zipfile.ZIP_DEFLATED
            output.writestr(item, data)


def verify_archive(archive, app, notes):
    expected = payload(app, notes)
    with zipfile.ZipFile(archive) as output:
        entries = output.infolist()
        if output.testzip() is not None or len(entries) != len(expected) or set(output.namelist()) != set(expected):
            raise ValueError('压缩包校验失败：文件集合或内容损坏')
        for entry in entries:
            mode, data = expected[entry.filename]
            if output.read(entry) != data or entry.external_attr >> 16 != mode or not entry.flag_bits & 0x800:
                raise ValueError('压缩包校验失败：文件内容、中文路径或执行权限不一致')


def publish(staging, output, names):
    previous = output / '.previous' / str(time.time_ns())
    moved, installed = [], []
    try:
        for name in names:
            old = output / name
            if old.exists() or old.is_symlink():
                previous.mkdir(parents=True, exist_ok=True)
                old.rename(previous / name)
                moved.append(name)
        for name in names:
            (staging / name).rename(output / name)
            installed.append(name)
    except BaseException:
        for name in reversed(installed):
            (output / name).rename(staging / name)
        for name in reversed(moved):
            (previous / name).rename(output / name)
        raise


def package_app(source, output, runner=subprocess.run):
    source, output = Path(source).resolve(), Path(output).resolve()
    validate_app(source)
    if output == source or output.is_relative_to(source):
        raise ValueError('输出目录不能位于应用内部')
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.macos-package-', dir=output) as temporary:
        staging = Path(temporary)
        app = staging / APP_NAME
        shutil.copytree(source, app, symlinks=True)
        if inventory(source) != inventory(app):
            raise ValueError('应用复制完整性校验失败')
        runner(['codesign','--force','--deep','--sign','-','--identifier',IDENTIFIER,str(app)], check=True)
        runner(['codesign','--verify','--deep','--strict',str(app)], check=True)
        notes = staging / NOTES_NAME
        notes.write_text(OPENING_GUIDE, encoding='utf-8')
        notes.chmod(0o644)
        archive = staging / ARCHIVE_NAME
        write_archive(archive, app, notes)
        verify_archive(archive, app, notes)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (staging / (ARCHIVE_NAME + '.sha256')).write_text(digest + '  ' + ARCHIVE_NAME + '\n', encoding='utf-8')
        publish(staging, output, [APP_NAME, NOTES_NAME, ARCHIVE_NAME, ARCHIVE_NAME + '.sha256'])
    return output / ARCHIVE_NAME


def main():
    parser = argparse.ArgumentParser(description='生成带中文首次打开说明的苹果桌面分发包')
    parser.add_argument('--app', default='target/release/bundle/macos/' + APP_NAME)
    parser.add_argument('--output', default='artifacts')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('苹果签名与分发打包必须在苹果桌面系统执行')
    archive = package_app(args.app, args.output)
    print('临时签名、应用复制、压缩内容、中文路径与执行权限校验通过')
    print('首次打开中文说明已随包分发：' + str(archive))
    print('压缩包大小：' + str(archive.stat().st_size) + ' 字节')


if __name__ == '__main__':
    main()
