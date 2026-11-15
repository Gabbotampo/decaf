import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
_SPEC = importlib.util.spec_from_file_location(
    "unmodelled_calls_tool", ROOT / "tools" / "unmodelled_calls.py"
)
TOOL = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = TOOL
_before = list(sys.path)
_SPEC.loader.exec_module(TOOL)
sys.path[:] = _before


class UnmodelledCallsTests(unittest.TestCase):
    def test_aggregate_counts_jars_calls_and_sole_rows(self):
        corpora = {
            "wide": [
                {"verdict": "ran-hollow", "calls": [["x", 2]], "unknown": 2},
                {"verdict": "ran-hollow", "calls": [["x", 1], ["y", 3]], "unknown": 4},
                {"verdict": "ran-clean", "calls": [], "unknown": 0},
                {"verdict": "refused"},
                {"verdict": "skipped"},
            ],
            "mega": [
                {"verdict": "ran-hollow", "calls": [["y", 5]], "unknown": 5},
            ],
        }
        totals, ranked = TOOL.aggregate(corpora)

        self.assertEqual(totals["wide"], {
            "ran-hollow": 2, "ran-clean": 1, "refused": 1, "skipped": 1,
            "jars": 5, "unknown_total": 6,
        })
        self.assertEqual(totals["mega"]["unknown_total"], 5)
        by_call = {row["call"]: row for row in ranked}
        # x is the only distinct unknown of one jar; y of another. Both are
        # hit by two jars, so total volume breaks the tie.
        self.assertEqual([row["call"] for row in ranked], ["y", "x"])
        self.assertEqual(by_call["x"], {
            "call": "x", "jars": 2, "calls": 3, "sole": 1, "corpora": ["wide"],
        })
        self.assertEqual(by_call["y"], {
            "call": "y", "jars": 2, "calls": 8, "sole": 1,
            "corpora": ["mega", "wide"],
        })

    def test_a_jar_with_two_distinct_calls_is_nobodys_sole_row(self):
        corpora = {"wide": [
            {"verdict": "ran-hollow", "calls": [["a", 4], ["b", 1]], "unknown": 5},
        ]}
        _, ranked = TOOL.aggregate(corpora)
        self.assertEqual([row["sole"] for row in ranked], [0, 0])

    def test_calls_counted_before_a_refusal_are_not_ranked(self):
        # A refused jar keeps what it counted before it stopped, for reading;
        # the totals and ranking stay over runs that finished.
        corpora = {"wide": [
            {"verdict": "refused", "counted_before_error": [["z", 3]]},
            {"verdict": "ran-hollow", "calls": [["a", 1]], "unknown": 1},
        ]}
        totals, ranked = TOOL.aggregate(corpora)
        self.assertEqual(totals["wide"]["unknown_total"], 1)
        self.assertEqual([row["call"] for row in ranked], ["a"])

    def test_parked_threads_rank_by_jars_then_threads(self):
        watcher = "com/electronwill/nightconfig/core/file/FileWatcher$FsWatcher"
        corpora = {
            "mega": [
                {"jar": "a.jar", "verdict": "ran-hollow",
                 "parked_threads": [{"name": "w-1", "runnable": watcher},
                                    {"name": "w-2", "runnable": watcher}]},
                {"jar": "b.jar", "verdict": "refused",
                 "parked_threads": [{"name": None, "runnable": "demo/Updater"}]},
                {"jar": "c.jar", "verdict": "ran-clean"},
            ],
            "wide": [
                {"jar": "a.jar", "verdict": "ran-hollow",
                 "parked_threads": [{"name": "w-1", "runnable": watcher}]},
            ],
        }
        # The same file name in two corpora is two jars; a jar starting two
        # watchers is still one jar.
        self.assertEqual(TOOL.parked_summary(corpora), [
            {"runnable": watcher, "jars": 2, "threads": 3},
            {"runnable": "demo/Updater", "jars": 1, "threads": 1},
        ])

    def test_parse_corpus_splits_name_and_directory(self):
        self.assertEqual(TOOL.parse_corpus("neoforge=/tmp/x"),
                         ("neoforge", Path("/tmp/x")))

    def test_locked_neoforge_reads_the_committed_file_names(self):
        locked = TOOL.locked_neoforge()
        self.assertTrue(locked)
        self.assertIn("BiomesOPlenty-neoforge-26.3-26.3.0.0.7.jar", locked)


    def _fabric_jar(self, entrypoints: dict, environment=None) -> Path:
        import json
        import tempfile
        import zipfile
        directory = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: __import__("shutil").rmtree(directory, ignore_errors=True))
        path = directory / "demo.jar"
        with zipfile.ZipFile(path, "w") as archive:
            metadata = {"id": "demo", "version": "1", "entrypoints": entrypoints}
            if environment is not None:
                metadata["environment"] = environment
            archive.writestr("fabric.mod.json", json.dumps(metadata))
        return path

    def test_a_fabric_jar_runs_main_then_server(self):
        jar = self._fabric_jar({"server": ["demo.S"], "main": ["demo.M"]})
        self.assertEqual(TOOL.entrypoint_kind(jar, "corpus-xl"), "main,server")
        self.assertEqual(TOOL.entrypoint_kind(self._fabric_jar({"server": ["demo.S"]}),
                                              "corpus-xl"), "server")
        self.assertEqual(TOOL.entrypoint_kind(self._fabric_jar({"main": ["demo.M"]}),
                                              "corpus-xl"), "main")
        self.assertIsNone(TOOL.entrypoint_kind(self._fabric_jar({"client": ["demo.C"]}),
                                               "corpus-xl"))

    def test_client_classification_keeps_roles_and_skip_reasons(self):
        client = self._fabric_jar({"client": ["demo.C"]})
        self.assertEqual(TOOL.classify_client(client, "corpus-xl"),
                         ("client-only", "client", "client entrypoint only"))
        dual = self._fabric_jar({"main": ["demo.M"], "client": ["demo.C"],
                                 "server": ["demo.S"]})
        self.assertEqual(TOOL.classify_client(dual, "corpus-xl"),
                         ("dual-client", "main,client", ""))
        server = self._fabric_jar({"main": ["demo.M"], "server": ["demo.S"]})
        self.assertEqual(TOOL.classify_client(server, "corpus-xl"),
                         ("library", None, "no client entrypoint"))

    def test_fabric_environment_client_is_client_even_without_a_client_key(self):
        import json
        import tempfile
        import zipfile
        directory = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: __import__("shutil").rmtree(directory, ignore_errors=True))
        path = directory / "demo.jar"
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr("fabric.mod.json", json.dumps({
                "id": "demo", "version": "1", "environment": "client",
                "entrypoints": {"main": ["demo.Main"]},
            }))
        self.assertEqual(TOOL.classify_client(path, "corpus-xl"),
                         ("client-only", "main", "environment: client"))
        self.assertEqual(TOOL.classify_server_skip(path, "corpus-xl"), "client-only")

    def test_fabric_environment_matches_the_physical_side(self):
        for environment in ("*", "client"):
            jar = self._fabric_jar({"client": ["demo.C"]}, environment)
            self.assertEqual(TOOL.entrypoint_kind(jar, "corpus-xl", "client"), "client")
        server = self._fabric_jar({"main": ["demo.M"], "client": ["demo.C"]}, "server")
        self.assertIsNone(TOOL.entrypoint_kind(server, "corpus-xl", "client"))
        self.assertEqual(TOOL.classify_client(server, "corpus-xl"),
                         ("server-only", None, "environment: server"))

    def test_neoforge_toml_and_dist_side_rules(self):
        import struct
        import tempfile
        import zipfile
        directory = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: __import__("shutil").rmtree(directory, ignore_errors=True))

        def mod_class(dists=()):
            u = lambda text: b"\x01" + struct.pack(">H", len(text)) + text.encode()
            pool = [u("example/ExampleMod"), b"\x07\x00\x01", u("java/lang/Object"), b"\x07\x00\x03",
                    u("Lnet/neoforged/fml/common/Mod;"), u("RuntimeVisibleAnnotations"),
                    u("value"), u("demo"), u("dist"), u("Lnet/neoforged/api/distmarker/Dist;"),
                    u("CLIENT"), u("DEDICATED_SERVER")]
            annotation = struct.pack(">H", 5)
            pairs = [(7, b"s" + struct.pack(">H", 8))]
            if dists:
                pairs.append((9, b"[" + struct.pack(">H", len(dists)) + b"".join(
                    b"e" + struct.pack(">HH", 10, 11 if d == "CLIENT" else 12) for d in dists)))
            annotation += struct.pack(">H", len(pairs))
            annotation += b"".join(struct.pack(">H", n) + value for n, value in pairs)
            attr = struct.pack(">HI", 6, len(annotation) + 2) + struct.pack(">H", 1) + annotation
            return (b"\xca\xfe\xba\xbe" + struct.pack(">HHH", 0, 52, 13) + b"".join(pool)
                    + struct.pack(">HHHHHH", 0x21, 2, 4, 0, 0, 0) + struct.pack(">H", 1) + attr)

        def jar(name, toml, mod, legacy=False):
            path = directory / name
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("META-INF/mods.toml" if legacy else "META-INF/neoforge.mods.toml", toml)
                archive.writestr("example/ExampleMod.class", mod)
            return path

        # Distant Horizons' shape: a mod-scoped property in NeoForge's file is
        # ordinary mod metadata, not the physical-side selector.
        client_toml = jar("client.jar", '[[mods]]\nmodId = "dh"\nclientSideOnly = "true"\nside = "BOTH"\n', mod_class())
        self.assertEqual(TOOL.classify_client(client_toml, "neoforge")[0], "neo-client")
        metadata = TOOL.ledger.read_jar_entrypoints(client_toml, force_neoforge=True)
        self.assertFalse(metadata.client_side_only)
        legacy = jar("legacy.jar", 'clientSideOnly = true\n[[mods]]\nmodId = "legacy"\n', mod_class(), legacy=True)
        self.assertEqual(TOOL.classify_client(legacy, "neoforge")[0], "client-only")
        dual = jar("dual.jar", "", mod_class(("CLIENT", "DEDICATED_SERVER")))
        self.assertEqual(TOOL.classify_client(dual, "neoforge")[0], "neo-client")

    def test_the_row_takes_the_probe_verdict(self):
        self.assertEqual(TOOL.row_verdict({"verdict": "ran-partial",
                                           "unknown_host_call_count": 0}), "ran-partial")
        self.assertEqual(TOOL.row_verdict({"unknown_host_call_count": 0}), "ran-clean")
        self.assertEqual(TOOL.row_verdict({"unknown_host_call_count": 3}), "ran-hollow")


if __name__ == "__main__":
    unittest.main()
