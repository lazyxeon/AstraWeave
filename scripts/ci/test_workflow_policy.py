import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "ci"))
import workflow_policy_cases

if __name__ == "__main__":
    workflow_policy_cases.unittest.main(module=workflow_policy_cases)
