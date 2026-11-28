"""Controls for the fixture's exact explicit-construction replay."""

import json
from pathlib import Path
import runpy
import unittest

from corpus_oracles import Unavailable


FIXTURE = Path(__file__).resolve().parent
EXPLICIT = runpy.run_path(str(FIXTURE / "explicit_recovery.py"))
MANIFEST = json.loads((FIXTURE / "decbench.json").read_text())
SCENARIOS = json.loads((FIXTURE / "scenarios.json").read_text())["run"]
MEMBER = MANIFEST["methods"][0]["member"]
CONTROL = """package fixture;

public final class AllocationStages {
    public static void run() {
        int marker = 10;
        try {
            int prepared = AllocationStages.prepare();
            marker = 15;
            AllocationItem receiver = allocate-instance AllocationItem;
            marker = 20;
            int actualArgument = AllocationStages.argument(marker);
            marker = 30;
            init-direct receiver, Lfixture/AllocationItem;-><init>(III)V, [prepared, actualArgument, marker];
            AllocationStages.publish(receiver);
        } catch (Throwable actualFault) {
            AllocationStages.caught(marker, actualFault);
        }
    }
}
"""


class ExplicitConstruction(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = EXPLICIT["RAW"]["prove"](
            FIXTURE / "classes.dex", MANIFEST, SCENARIOS, values=EXPLICIT["VALUES"],
        )["checks"]

    def compare(self, text, input_index=0, scenario_index=0):
        parsed = EXPLICIT["parse"](text, MEMBER)
        spec = SCENARIOS[scenario_index]
        observed, error = EXPLICIT["run_scenario"](parsed, spec, EXPLICIT["VALUES"][input_index])
        return observed.events, error, self.raw[input_index][scenario_index]["observed"]

    def test_control_preserves_all_scenarios_at_both_operand_pairs(self):
        for pair in range(len(EXPLICIT["VALUES"])):
            for scenario in range(len(SCENARIOS)):
                with self.subTest(pair=pair, scenario=scenario):
                    events, error, original = self.compare(CONTROL, pair, scenario)
                    self.assertIsNone(error)
                    self.assertEqual(events, original)

    def test_discarded_extra_allocation_is_visible_on_warm_retry(self):
        wrong = CONTROL.replace(
            "            AllocationStages.publish(receiver);",
            "            AllocationStages.publish(receiver);\n"
            "            AllocationItem extra = allocate-instance AllocationItem;",
        )
        parsed = EXPLICIT["parse"](wrong, MEMBER)
        case = next(index for index, spec in enumerate(SCENARIOS)
                    if spec["id"] == "second-construction-does-not-repeat-class-init")
        observed, error = EXPLICIT["run_scenario"](parsed, SCENARIOS[case], EXPLICIT["VALUES"][0])
        raw = self.raw[0][case]
        self.assertIsNone(error)
        self.assertEqual(observed.events, raw["observed"])
        self.assertNotEqual(observed.operations, raw["operations"])
        self.assertEqual(sum(op["op"] == "allocate" for op in observed.operations), 4)
        self.assertEqual(sum(op["op"] == "allocate" for op in raw["operations"]), 2)

    def test_exact_receiver_links_allocation_initializer_and_publication(self):
        parsed = EXPLICIT["parse"](CONTROL, MEMBER)
        case = next(index for index, spec in enumerate(SCENARIOS)
                    if spec["id"] == "second-construction-does-not-repeat-class-init")
        observed, error = EXPLICIT["run_scenario"](parsed, SCENARIOS[case], EXPLICIT["VALUES"][0])
        self.assertIsNone(error)
        self.assertEqual(observed.operations, self.raw[0][case]["operations"])
        allocated = [op["result"] for op in observed.operations if op["op"] == "allocate"]
        constructed = [op["receiver"] for op in observed.operations
                       if op["op"] == "call" and op["member"][1] == "<init>"]
        published = [op["arguments"][0] for op in observed.operations
                     if op["op"] == "call" and op["member"][1] == "publish"]
        self.assertEqual(allocated, constructed)
        self.assertEqual(allocated, published)
        self.assertNotEqual(allocated[0], allocated[1])

    def test_same_numeric_literal_does_not_establish_operand_origin(self):
        wrong = CONTROL.replace("[prepared, actualArgument, marker]", "[7, actualArgument, marker]")
        events, error, original = self.compare(wrong)
        self.assertIsNone(error)
        self.assertEqual(events, original)
        events, error, original = self.compare(wrong, 1)
        self.assertIsNone(error)
        self.assertNotEqual(events, original)
        self.assertIn("constructor:7:109:30", events)

    def test_swapped_operands_are_observable(self):
        wrong = CONTROL.replace("[prepared, actualArgument, marker]", "[actualArgument, prepared, marker]")
        events, error, original = self.compare(wrong)
        self.assertIsNone(error)
        self.assertNotEqual(events, original)
        self.assertIn("constructor:11:7:30", events)

    def test_commit_before_construction_is_observable(self):
        wrong = CONTROL.replace("            marker = 30;\n", "").replace("actualArgument, marker]", "actualArgument, 30]")
        events, error, original = self.compare(wrong, scenario_index=4)
        self.assertIsNone(error)
        self.assertNotEqual(events, original)
        self.assertIn("caught:20:constructor", events)

    def test_distinct_receiver_and_target_are_rejected(self):
        for old, new, diagnostic in [
            ("init-direct receiver,", "init-direct null,", "uninitialized receiver"),
            ("<init>(III)V", "<init>(II)V", "unexpected fixture initializer"),
        ]:
            with self.subTest(wrong=new):
                _, error, _ = self.compare(CONTROL.replace(old, new))
                self.assertIn(diagnostic, error)

    def test_narrowed_catch_escapes_original_initialization_error(self):
        wrong = CONTROL.replace("catch (Throwable actualFault)", "catch (Exception actualFault)")
        events, error, original = self.compare(wrong, scenario_index=2)
        self.assertIsNone(error)
        self.assertNotEqual(events, original)
        self.assertIn("escape:class-init", events)

    def test_unsupported_effect_does_not_claim_equivalence(self):
        wrong = CONTROL.replace("AllocationStages.publish(receiver);", "AllocationStages.teleport(receiver);")
        with self.assertRaises(Unavailable):
            EXPLICIT["parse"](wrong, MEMBER)


if __name__ == "__main__":
    unittest.main()
