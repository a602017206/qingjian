import importlib.util
from pathlib import Path
import tempfile,plistlib,unittest,zipfile
SPEC=importlib.util.spec_from_file_location('package_ios',Path('scripts/package-ios.py'))
MODULE=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
class PackagingTests(unittest.TestCase):
 def test_simulator_and_device_are_separate(self):
  with tempfile.TemporaryDirectory() as temporary:
   root=Path(temporary)
   for kind,platform in [('simulator','iPhoneSimulator'),('device','iPhoneOS')]:
    app=root/kind/'qingjian.xcarchive/Products/Applications/qingjian.app';app.mkdir(parents=True)
    with (app/'Info.plist').open('wb') as file:plistlib.dump({'CFBundleIdentifier':'com.qingjian.notebook','CFBundleSupportedPlatforms':[platform],'CFBundleExecutable':'qingjian'},file)
    (app/'qingjian').write_bytes(b'example');(app/'qingjian').chmod(0o755)
   for kind in ['simulator','device']:
    archive=MODULE.package_ios(root,root/'output',kind)
    with zipfile.ZipFile(archive) as output:
     prefix='Payload/' if kind=='device' else ''
     info=output.getinfo(prefix+'qingjian.app/qingjian');self.assertTrue(info.external_attr>>16&0o111)
     plist=plistlib.loads(output.read(prefix+'qingjian.app/Info.plist'))
     self.assertEqual(plist['CFBundleSupportedPlatforms'],['iPhoneOS' if kind=='device' else 'iPhoneSimulator'])
 def test_missing_or_ambiguous_application_is_rejected(self):
  with tempfile.TemporaryDirectory() as temporary:
   with self.assertRaises(ValueError):MODULE.package_ios(Path(temporary),Path(temporary)/'out','device')
if __name__=='__main__':unittest.main()
