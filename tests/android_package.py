import importlib.util, pathlib, struct, unittest
spec=importlib.util.spec_from_file_location('android_check',pathlib.Path(__file__).parents[1]/'scripts/check-android.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
def elf(alignment=16384,offset=0,address=0):
    data=bytearray(120);data[:6]=b'\x7fELF\x02\x01';struct.pack_into('<H',data,18,183);struct.pack_into('<Q',data,32,64);struct.pack_into('<HH',data,54,56,1);struct.pack_into('<IIQQQQQQ',data,64,1,5,offset,address,0,120,120,alignment);return bytes(data)
class Alignment(unittest.TestCase):
    def test_compatible(self):module.verify_elf(elf(),'本地库')
    def test_legacy_alignment(self):
        with self.assertRaisesRegex(ValueError,'16 KB'):module.verify_elf(elf(4096),'旧库')
    def test_mismatched_offsets(self):
        with self.assertRaises(ValueError):module.verify_elf(elf(16384,0,4096),'错位库')
    def test_invalid_file(self):
        for data in [b'',b'not-elf',elf()[:80]]:
            with self.assertRaises(ValueError):module.verify_elf(data,'截断库')
if __name__=='__main__':unittest.main()
