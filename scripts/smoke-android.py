import os,subprocess,time,pathlib,sys,re

def verify_credential_stage(log,stage):
    expected=['SAVED','RESTORED_AND_DELETED','FORGOTTEN'][stage]
    events=re.findall(r'QingjianCredentialSmoke\s*:\s*(\w+)',log)
    if 'FAILED' in events or expected not in events:
        raise RuntimeError('安卓安全认证进程重启验证失败，阶段：'+str(stage+1))

def main():
    if os.environ.get('GITHUB_ACTIONS')!='true':raise RuntimeError('安卓原生启动测试仅在云端执行')
    output=pathlib.Path('test-results/android-startup');output.mkdir(parents=True,exist_ok=True)
    package='com.qingjian.notebook'
    def adb(*args,timeout=60,missing_ok=False):
        result=subprocess.run(['adb',*args],capture_output=True,timeout=timeout)
        if result.returncode and not missing_ok:raise RuntimeError('安卓测试命令失败：'+result.stderr.decode(errors='replace'))
        return result.stdout
    try:
        page_size=adb('shell','getconf','PAGE_SIZE').decode().strip()
        if page_size!=os.environ['EXPECTED_PAGE_SIZE']:raise RuntimeError('模拟器实际页大小与测试目标不符：'+page_size)
        adb('install','-r',sys.argv[1]);adb('logcat','-c')
        for attempt in range(3):
            adb('shell','am','force-stop',package)
            adb('logcat','-c')
            launch=adb('shell','am','start','-W','-n',package+'/.MainActivity').decode(errors='replace')
            (output/f'launch-{attempt}.txt').write_text(launch)
            if 'Error:' in launch:raise RuntimeError('安卓启动入口失败')
            time.sleep(10)
            if not adb('shell','pidof',package,missing_ok=True).strip():raise RuntimeError('安卓应用启动后退出')
            stage_log=adb('logcat','-d').decode(errors='replace')
            (output/f'credential-stage-{attempt}.txt').write_text(stage_log)
            verify_credential_stage(stage_log,attempt)
        ready=False
        for _ in range(6):
            adb('shell','uiautomator','dump','/sdcard/qingjian-window.xml')
            xml=adb('shell','cat','/sdcard/qingjian-window.xml').decode(errors='replace');(output/'window.xml').write_text(xml)
            if '个人笔记' in xml and '工作台' in xml:ready=True;break
            time.sleep(5)
        if not ready:raise RuntimeError('安卓进程存活，但未进入笔记工作台')
        crashes=adb('logcat','-d','-b','crash').decode(errors='replace')
        if package in crashes:raise RuntimeError('安卓启动日志存在应用崩溃')
        print('安卓三次冷启动、认证保存重启读取与忘记、笔记工作台及进程存活验证通过，页大小：'+page_size)
    finally:
        for args,name in [(('logcat','-d'),'logcat.txt'),(('logcat','-d','-b','crash'),'crash.txt')]:
            try:(output/name).write_bytes(adb(*args))
            except Exception as error:(output/name).write_text(str(error))
        try:(output/'screen.png').write_bytes(adb('exec-out','screencap','-p'))
        except Exception:pass

if __name__=='__main__':main()
