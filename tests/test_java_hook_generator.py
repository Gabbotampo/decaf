import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / "tools" / "java_hook_generator.py"
spec = importlib.util.spec_from_file_location("java_hook_generator", TOOL)
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)


class JavaHookGeneratorTests(unittest.TestCase):
    def setUp(self):
        self.mapping = {
            "rows": [
                {"vanilla": "net.minecraft.world.entity.player.Player#attack", "descriptor": "(LEntity;)V", "sunrise": "sunrise_client::game::melee::Game::attack", "anchor": "HEAD; cancellable result", "fit": "EXACT FIT", "evidence": "point"},
                {"vanilla": "Minecraft#continueAttack", "descriptor": "(Z)V", "sunrise": "attack_entity", "anchor": "HOOK", "fit": "FITS WITH A STATED DIFFERENCE", "evidence": "server dispatch difference"},
                {"vanilla": "Minecraft#startAttack", "descriptor": "()Z", "sunrise": "none", "anchor": "HOOK", "fit": "NO FIT", "evidence": "none"},
            ]
        }
        target = "sunrise_client::game::melee::Game::attack"
        self.functions = {"functions": [{"path": target}]}
        self.points = {"points": [{"path": target, "api_refusal": None, "signature_hash": "abc123"}]}

    def test_exact_point_and_stated_difference_hook_emit(self):
        plan = generator.generate(self.mapping, self.functions, self.points)
        self.assertEqual([item["kind"] for item in plan["inject"]], ["head"])
        self.assertEqual(plan["inject"][0]["hash"], "abc123")
        self.assertEqual(plan["hooks"][0]["target"], "attack_entity")
        text = generator.render(plan, "lane1.json")
        self.assertIn("[[inject]]", text)
        self.assertIn("[[hook]]", text)
        self.assertNotIn("start_attack", text)

    def test_refused_or_stale_points_fail_closed(self):
        target = self.mapping["rows"][0]["sunrise"]
        plan = generator.generate(self.mapping, {"functions": [{"path": target}]}, {"points": [{"path": target, "api_refusal": "Hit has no API type"}]})
        self.assertEqual(plan["inject"], [])
        self.assertTrue(any("refused point" in error for error in plan["errors"]))

    def test_cli_shape_is_deterministic(self):
        plan = generator.generate(self.mapping, self.functions, self.points)
        self.assertEqual(generator.render(plan, "lane1.json"), generator.render(plan, "lane1.json"))


if __name__ == "__main__":
    unittest.main()
