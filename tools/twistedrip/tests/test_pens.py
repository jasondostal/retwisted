"""decode/pens.py vs the RE repo's scripts/pens_extract.py oracle output."""
import json
import subprocess
import sys

from tools.twistedrip.decode import pens

from .conftest import RE_ROOT, module_resources, _require_private_checkout


def test_pens_byte_parity(tmp_path):
    _require_private_checkout()
    module = module_resources('message-mayhem')
    res = module.get(('Pens', 500))
    assert res is not None, 'no Pens 500 resource in Message Mayhem'

    got = pens.decode(res.data)

    oracle_script = RE_ROOT / 'scripts' / 'pens_extract.py'
    out = tmp_path / 'oracle_pens500.json'
    subprocess.run([sys.executable, str(oracle_script), str(out)],
                    cwd=RE_ROOT, check=True, capture_output=True)
    oracle = json.loads(out.read_text())

    assert json.dumps(got, sort_keys=True) == json.dumps(oracle, sort_keys=True)
