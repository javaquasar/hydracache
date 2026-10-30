import unittest
from runner import account

class ActualNativeGrammar(unittest.TestCase):
    def test_diagnostic_between_header_and_outcome(self):
        for middle in ('ok\n','HC074_TABLE:{}\nok\n'):
            text='running 1 test\ntest module::case ... '+middle+'\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.1s\n'
            account(text.encode(),list(range(10)),'module::case',False)

    def test_missing_duplicate_wrong_headers_or_summary_refuse(self):
        valid='test module::case ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out;\n'
        for body in (valid.replace('module::case','module::other'),valid.replace('test module::case ... ok\n',''),
          'test module::case ... ok\n'+valid,valid+valid,valid.replace('1 passed','0 passed'),
          valid.replace('0 ignored','1 ignored'),valid.replace('9 filtered','8 filtered')):
            with self.subTest(body=body),self.assertRaises(ValueError):account(body.encode(),list(range(10)),'module::case',False)

    def test_known_negative_never_becomes_pass(self):
        raw=b'test module::case ... FAILED\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out;\n'
        account(raw,list(range(10)),'module::case',True)
        with self.assertRaises(ValueError):account(raw,list(range(10)),'module::case',False)

if __name__=='__main__':unittest.main()
