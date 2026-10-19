import importlib.util
from pathlib import Path
import plistlib
import subprocess
import tempfile
import unittest
import zipfile

SPEC = importlib.util.spec_from_file_location('package_macos', Path('scripts/package-macos.py'))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

class DistributionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.app = self.root / '轻笺.app'
        self.output = self.root / 'output'
        binary = self.app / 'Contents/MacOS/qingjian'
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b'example executable')
        binary.chmod(0o755)
        with (self.app / 'Contents/Info.plist').open('wb') as file:
            plistlib.dump({'CFBundleExecutable':'qingjian','CFBundleIdentifier':'com.qingjian.notebook'},file)
        (self.app / 'Contents/图标链接').symlink_to('Info.plist')
        self.calls = []

    def runner(self, args, **kwargs):
        self.calls.append(args)
        return subprocess.CompletedProcess(args, 0)

    def test_package_contains_chinese_instructions_and_preserves_permissions(self):
        archive = MODULE.package_app(self.app, self.output, runner=self.runner)
        self.assertGreaterEqual(len(self.calls),2)
        self.assertEqual(self.calls[0][:5], ['codesign','--force','--deep','--sign','-'])
        self.assertEqual(self.calls[1][:4], ['codesign','--verify','--deep','--strict'])
        with zipfile.ZipFile(archive) as zipped:
            notes = zipped.read('首次打开说明.txt').decode('utf-8')
            self.assertIn('隐私与安全性',notes)
            self.assertIn('仍要打开',notes)
            self.assertIn('临时签名',notes)
            self.assertNotIn('xattr',notes)
            info = zipped.getinfo('轻笺.app/Contents/MacOS/qingjian')
            self.assertTrue(info.external_attr >> 16 & 0o111)
            self.assertTrue(info.flag_bits & 0x800)
            self.assertEqual(zipped.read('轻笺.app/Contents/图标链接'),b'Info.plist')
        MODULE.verify_archive(archive,self.output / '轻笺.app',self.output / '首次打开说明.txt')
        self.assertIn(archive.name,(self.output / (archive.name+'.sha256')).read_text())
        self.assertEqual((self.app / 'Contents/MacOS/qingjian').read_bytes(),b'example executable')

    def test_signing_failure_keeps_previous_distribution(self):
        self.output.mkdir()
        old = self.output / '轻笺-苹果桌面.zip'
        old.write_bytes(b'previous package')
        def fail(args, **kwargs):
            if '--verify' in args:
                raise subprocess.CalledProcessError(1,args)
            return self.runner(args,**kwargs)
        with self.assertRaises(subprocess.CalledProcessError):
            MODULE.package_app(self.app,self.output,runner=fail)
        self.assertEqual(old.read_bytes(),b'previous package')
        self.assertFalse((self.output/'首次打开说明.txt').exists())

    def test_archive_verification_rejects_modified_bytes(self):
        self.output.mkdir()
        archive = self.output / 'broken.zip'
        notes = self.output / '首次打开说明.txt'
        notes.write_text('首次打开说明',encoding='utf-8')
        with zipfile.ZipFile(archive,'w') as zipped:
            zipped.writestr('unexpected.txt','changed')
        with self.assertRaises(ValueError):
            MODULE.verify_archive(archive,self.output/'轻笺.app',self.output/'首次打开说明.txt')

    def test_invalid_application_is_rejected_before_signing(self):
        (self.app/'Contents/MacOS/qingjian').chmod(0o644)
        with self.assertRaises(ValueError):
            MODULE.package_app(self.app,self.output,runner=self.runner)
        self.assertEqual(self.calls,[])

if __name__ == '__main__':
    unittest.main()
