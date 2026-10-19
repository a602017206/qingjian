from pathlib import Path
from datetime import datetime
import argparse,os,plistlib,stat,zipfile

def package_ios(root,output,kind):
    if kind not in ('simulator','device'):
        raise ValueError('苹果移动端产物类型无效')
    platform='iPhoneOS' if kind=='device' else 'iPhoneSimulator'
    candidates=[]
    for app in root.rglob('*.app'):
        info=app/'Info.plist'
        if not info.is_file():continue
        with info.open('rb') as file:metadata=plistlib.load(file)
        if metadata.get('CFBundleIdentifier')=='com.qingjian.notebook' and metadata.get('CFBundleSupportedPlatforms')==[platform]:candidates.append(app)
    archives=[app for app in candidates if '.xcarchive/Products/Applications/' in app.as_posix()]
    if archives:candidates=archives
    if len(candidates)!=1:raise ValueError('应当只有一份对应平台的苹果应用，不能按修改时间猜测')
    app=candidates[0]
    with (app/'Info.plist').open('rb') as file:info=plistlib.load(file)
    executable=info.get('CFBundleExecutable','')
    if not executable or Path(executable).name!=executable or not (app/executable).is_file() or not (app/executable).stat().st_mode&0o111:raise ValueError('苹果移动应用可执行文件或权限无效')
    output.mkdir(parents=True,exist_ok=True)
    archive=output/('qingjian-ios-unsigned-arm64.ipa' if kind=='device' else 'qingjian-ios-simulator-arm64.zip')
    prefix='Payload/' if kind=='device' else ''
    expected={}
    for path in sorted(app.rglob('*')):
        if path.is_symlink():
            if not path.resolve().is_relative_to(app.resolve()):raise ValueError('应用链接不能指向包外')
            data=os.readlink(path).encode()
        elif path.is_file():data=path.read_bytes()
        elif path.is_dir():continue
        else:raise ValueError('应用文件类型无效')
        expected[prefix+app.name+'/'+path.relative_to(app).as_posix()]=(path.lstat().st_mode,data)
    temporary=archive.with_suffix(archive.suffix+'.tmp')
    try:
        with zipfile.ZipFile(temporary,'w',compression=zipfile.ZIP_DEFLATED) as zipped:
            for name,(mode,data) in expected.items():
                entry=zipfile.ZipInfo(name,datetime.now().timetuple()[:6]);entry.create_system=3;entry.external_attr=mode<<16;entry.compress_type=zipfile.ZIP_DEFLATED;zipped.writestr(entry,data)
        with zipfile.ZipFile(temporary) as zipped:
            if zipped.testzip() or set(zipped.namelist())!=set(expected):raise ValueError('苹果移动应用压缩校验失败')
            for name,(mode,data) in expected.items():
                if zipped.read(name)!=data or zipped.getinfo(name).external_attr>>16!=mode:raise ValueError('苹果移动应用内容或权限不一致')
        temporary.replace(archive)
    finally:
        temporary.unlink(missing_ok=True)
    return archive

if __name__=='__main__':
    parser=argparse.ArgumentParser(description='区分苹果模拟器与未签名真机产物')
    parser.add_argument('--kind',choices=['simulator','device'],default='simulator')
    parser.add_argument('--root',default='src-tauri/gen/apple')
    parser.add_argument('--output',default='artifacts/release')
    args=parser.parse_args()
    print('苹果移动应用已打包并核对：'+str(package_ios(Path(args.root),Path(args.output),args.kind)))
