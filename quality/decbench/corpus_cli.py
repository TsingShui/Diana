"""Command handlers for opt-in public APK assessments."""

import json

import corpus_assets
from evidence import write_json


def list_projects():
    print(json.dumps([
        {"id": project.id, "name": project.name, "version": project.version,
         "source_revision": project.source_commit}
        for project in corpus_assets.load_projects().values()
    ], indent=2))
    return 0


def prepare(args, directory):
    project = corpus_assets.project_by_id(args.corpus)
    assets = corpus_assets.prepare_assets(project, directory, download=args.download,
                                          apk=args.apk, source=args.source)
    print(json.dumps({"apk": str(assets.apk), "source": str(assets.source)}, indent=2))
    return 0


def discover(args):
    import corpus_discover
    import corpus_eval

    project = corpus_assets.project_by_id(args.corpus)
    directory = args.prepared or args.state / "public-corpora"
    assets = corpus_assets.prepare_assets(project, directory)
    _, reviewed = corpus_eval.case_configuration(project, None)
    result = corpus_discover.discover(
        assets.apk, assets.source, project.source_root, project.source_commit,
        project.provenance["source_repository_url"], limit=args.limit,
        input_sha256=reviewed["input_sha256"], revision_entry=reviewed.get("revision_entry"),
        exclude=[method["at"] for method in reviewed["methods"]], shard=args.shard,
    )
    if not result["methods"]:
        raise ValueError(f"{project.id}: discovery did not establish a source-aligned method")
    number, count = args.shard
    suffix = f"-{number}-of-{count}" if count != 1 else ""
    path = args.output or args.state / "discovery" / f"{project.id}-{args.limit}{suffix}.json"
    write_json(path, result)
    print(path)
    return 0


def judge(args, directory):
    import corpus_eval

    if args.input or args.suite or args.inventory_only or args.declared_only:
        raise ValueError("--corpus selects its own reviewed methods; use --at to narrow them")
    if args.backend != "diana":
        raise ValueError("use --binary or --jadx-jar to select a public corpus backend")
    project = corpus_assets.project_by_id(args.corpus)
    binary, backend = (args.jadx_jar, "jadx") if args.jadx_jar else (args.binary, "diana")
    return corpus_eval.run(project, directory, binary, args.revision, backend,
                           args.at, args.jobs, args.timeout, execute_models=args.execute_models,
                           case_path=args.cases, inspect_structure=args.inspect_structure,
                           checkpoint=args.state / "checkpoints" if args.resume else None)


def campaign(args):
    import corpus_campaign

    directory = args.prepared or args.state / "public-corpora"
    output = args.output or args.state / "campaigns"
    receipt, result = corpus_campaign.run(
        args.corpus, directory, output, args.state, args.binary,
        jadx_jar=args.jadx_jar, revision=args.revision, profile=args.profile, limit=args.limit, shards=args.shards,
        execute_models=args.execute_models, inspect_structure=args.inspect_structure, resume=args.resume,
        jobs=args.jobs, timeout=args.timeout,
    )
    print(receipt)
    return result


def coverage(args):
    import corpus_coverage
    import corpus_eval

    project = corpus_assets.project_by_id(args.corpus)
    directory = args.prepared or args.state / "public-corpora"
    assets = corpus_assets.prepare_assets(project, directory)
    selections = [corpus_eval.case_configuration(project, path)[1] for path in (args.cases or [None])]
    reports = [json.loads(path.read_text(encoding="utf-8")) for path in args.report]
    result = corpus_coverage.measure(assets.apk, assets.source, project.source_root, cases=selections, reports=reports)
    result["corpus"] = project.id
    path = args.output or args.state / f"{project.id}-coverage.json"
    write_json(path, result)
    print(path)
    return corpus_coverage.result_code(result)


def time_builds(args):
    import corpus_eval
    import corpus_oracles
    import corpus_timing

    project = corpus_assets.project_by_id(args.corpus)
    directory = args.prepared or args.state / "public-corpora"
    assets = corpus_assets.prepare_assets(project, directory)
    case_path, config = corpus_eval.case_configuration(project, args.cases)
    verified = corpus_oracles.verify_cases(config, assets.apk, assets.source, project.source_root)
    cases = corpus_eval.select_cases(project, verified, args.at)
    unverified = [case["at"] for case in cases if any(
        item.get("status") == "SETUP_ERROR" for item in case["verified"]
    )]
    if unverified:
        raise ValueError(f"{project.id}: cannot time unverified methods: {', '.join(unverified)}")
    result = corpus_timing.measure(args.baseline, args.candidate, assets.apk, cases,
                                   args.repeats, args.timeout, source_archive=assets.source,
                                   time_budget=args.time_budget)
    result["corpus"] = {"id": project.id, "source_revision": project.source_commit, "cases": str(case_path)}
    path = args.output or args.state / f"{project.id}-timing.json"
    write_json(path, result)
    print(path)
    return 0 if result["comparable"] else 2
