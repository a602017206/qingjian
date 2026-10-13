from pathlib import Path
import shutil

apps = [path for path in Path('src-tauri/gen/apple').rglob('*.app') if path.is_dir()]
if not apps:
    raise SystemExit('没有找到模拟器应用，构建流程未完成')
app = max(apps, key=lambda path: path.stat().st_mtime)
Path('artifacts').mkdir(exist_ok=True)
shutil.make_archive('artifacts/qingjian-ios-simulator', 'zip', app.parent, app.name)
print('苹果模拟器应用打包完成')
