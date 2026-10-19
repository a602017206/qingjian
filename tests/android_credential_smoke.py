import importlib.util,pathlib,unittest
spec=importlib.util.spec_from_file_location('android_smoke',pathlib.Path(__file__).parents[1]/'scripts'/'smoke-android.py')
smoke=importlib.util.module_from_spec(spec);spec.loader.exec_module(smoke)
class CredentialSmokeTests(unittest.TestCase):
    def test_expected_stage_is_required(self):
        for stage,marker in enumerate(['SAVED','RESTORED_AND_DELETED','FORGOTTEN']):
            smoke.verify_credential_stage('I QingjianCredentialSmoke: '+marker,stage)
            with self.assertRaises(RuntimeError):smoke.verify_credential_stage('I QingjianCredentialSmoke: OTHER',stage)
    def test_failed_native_storage_never_counts_as_pass(self):
        with self.assertRaises(RuntimeError):smoke.verify_credential_stage('I QingjianCredentialSmoke: SAVED\nE QingjianCredentialSmoke: FAILED',0)
        with self.assertRaises(RuntimeError):smoke.verify_credential_stage('I AnotherApp: SAVED',0)
if __name__=='__main__':unittest.main()
