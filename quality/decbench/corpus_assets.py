"""Acquire explicitly selected public project inputs for offline benchmark runs."""

from dataclasses import dataclass
import gzip
import http.client
import json
import os
from pathlib import Path
import re
import stat
import tarfile
from tempfile import NamedTemporaryFile
from time import monotonic
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener
from zipfile import is_zipfile
from zlib import error as DeflateError


CATALOG_ROOT = Path(__file__).with_name("corpora")
MAX_MANIFEST_BYTES = 256 * 1024
MAX_ASSET_BYTES = 512 * 1024 * 1024
COPY_BYTES = 256 * 1024
DOWNLOAD_SECONDS = 600
SOCKET_SECONDS = 20
PROJECT_ID = re.compile(r"[a-z0-9][a-z0-9.-]{0,79}(?<!\.)")
COMMIT = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")
ARCHIVE_ROOT = re.compile(r"[A-Za-z0-9][A-Za-z0-9._+-]{0,255}")


@dataclass(frozen=True)
class Asset:
    url: str
    max_bytes: int
    sha256: str | None = None


@dataclass(frozen=True)
class Project:
    id: str
    name: str
    version: str
    apk: Asset
    source: Asset
    source_commit: str
    source_root: str
    manifest: Path
    provenance: dict


@dataclass(frozen=True)
class Assets:
    apk: Path
    source: Path


def https_url(value: str) -> None:
    parsed = urlsplit(value)
    if (parsed.scheme != "https" or not parsed.hostname or parsed.username is not None
            or parsed.fragment or any(character.isspace() for character in value)):
        raise ValueError("asset URL must be HTTPS without credentials or a fragment")


def asset(value: dict) -> Asset:
    if not isinstance(value, dict) or not isinstance(value.get("url"), str):
        raise ValueError("expected an asset URL and byte bound")
    https_url(value["url"])
    limit = value.get("max_bytes")
    if type(limit) is not int or not 0 < limit <= MAX_ASSET_BYTES:
        raise ValueError(f"asset byte bound must be between 1 and {MAX_ASSET_BYTES}")
    digest = value.get("sha256")
    if digest is not None and (not isinstance(digest, str) or not SHA256.fullmatch(digest)):
        raise ValueError("asset provenance digest must be a lowercase SHA-256")
    return Asset(value["url"], limit, digest)


def read_project(path: Path) -> Project:
    with path.open("rb") as stream:
        data = stream.read(MAX_MANIFEST_BYTES + 1)
    if len(data) > MAX_MANIFEST_BYTES:
        raise ValueError("project manifest exceeds its byte bound")
    value = json.loads(data)
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError("expected a version-1 project manifest")
    for name in ("id", "name", "version", "source_commit", "source_root"):
        if not isinstance(value.get(name), str) or not value[name].strip():
            raise ValueError(f"expected a nonempty {name}")
    if not PROJECT_ID.fullmatch(value["id"]) or value["id"] != path.stem:
        raise ValueError("project ID must match its manifest basename")
    commit = value["source_commit"]
    if not COMMIT.fullmatch(commit) or not ARCHIVE_ROOT.fullmatch(value["source_root"]):
        raise ValueError("expected a full source commit and a single archive root component")
    source = asset(value.get("source"))
    if not re.search(rf"(?<![0-9a-f]){commit}(?![0-9a-f])", urlsplit(source.url).path):
        raise ValueError("source archive URL must name the pinned commit")
    if not isinstance(value.get("provenance"), dict) or not value["provenance"]:
        raise ValueError("expected project provenance")
    return Project(value["id"], value["name"], value["version"], asset(value.get("apk")),
                   source, commit, value["source_root"], path, value["provenance"])


def load_projects(catalog_root: Path = CATALOG_ROOT) -> dict[str, Project]:
    """Read project pins and provenance without acquiring any artifacts."""
    projects = {}
    for path in sorted(Path(catalog_root).resolve().glob("*.json")):
        if path.name.endswith(".cases.json"):
            continue
        try:
            project = read_project(path)
        except (OSError, ValueError) as error:
            raise ValueError(f"{path}: {error}") from error
        projects[project.id] = project
    return projects


def project_by_id(identifier: str) -> Project:
    projects = load_projects()
    if identifier not in projects:
        raise ValueError(f"unknown public corpus: {identifier}")
    return projects[identifier]


class HttpsRedirects(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        https_url(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def open_download(url: str):
    https_url(url)
    request = Request(url, headers={"Accept-Encoding": "identity", "User-Agent": "Diana DecBench"})
    return build_opener(HttpsRedirects()).open(request, timeout=SOCKET_SECONDS)


def regular_file(path: Path, limit: int):
    # The nonblocking open keeps a substituted named pipe from hanging the importer.
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_BINARY", 0))
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode):
            raise ValueError(f"expected a regular file: {path}")
        if not 0 < info.st_size <= limit:
            raise ValueError(f"{path} must be nonempty and at most {limit} bytes")
        return os.fdopen(fd, "rb")
    except BaseException:
        os.close(fd)
        raise


def validate_container(stream, filename: str) -> None:
    stream.seek(0)
    if filename == "input.apk":
        if not is_zipfile(stream):
            raise ValueError("APK is not a readable ZIP container")
        return
    try:
        with gzip.GzipFile(fileobj=stream) as decompressed:
            header = decompressed.read(tarfile.BLOCKSIZE)
        tarfile.TarInfo.frombuf(header, "utf-8", "surrogateescape")
    except (OSError, EOFError, tarfile.HeaderError, DeflateError) as error:
        raise ValueError("source is not a readable gzip-compressed tar container") from error


def copy_bounded(source, destination, limit: int, deadline: float | None = None) -> int:
    total = 0
    while True:
        if deadline is not None and monotonic() >= deadline:
            raise TimeoutError(f"download exceeded {DOWNLOAD_SECONDS} seconds")
        chunk = source.read1(min(COPY_BYTES, limit + 1 - total))
        if not chunk:
            return total
        total += len(chunk)
        if total > limit:
            raise ValueError(f"asset exceeds the {limit}-byte limit")
        destination.write(chunk)


def fetch(pin: Asset, destination) -> None:
    deadline = monotonic() + DOWNLOAD_SECONDS
    with open_download(pin.url) as source:
        length = source.headers.get("Content-Length")
        if length is not None:
            length = int(length)
            if not 0 < length <= pin.max_bytes:
                raise ValueError(f"download must be nonempty and at most {pin.max_bytes} bytes")
        received = copy_bounded(source, destination, pin.max_bytes, deadline)
        if not received or length is not None and received != length:
            raise ValueError("download ended before the announced asset was received")


def prepare_asset(destination: Path, pin: Asset, local: Path | None, download: bool) -> Path:
    if destination.is_symlink():
        raise ValueError(f"cached asset must not be a symlink: {destination}")
    if local is None or local.resolve() == destination:
        try:
            with regular_file(destination, pin.max_bytes) as stream:
                validate_container(stream, destination.name)
        except FileNotFoundError:
            if not download or local is not None:
                raise ValueError(f"missing {destination}; use --download or import a local copy") from None
        except (OSError, ValueError) as error:
            if not download or local is not None:
                raise ValueError(
                    f"invalid cached asset {destination}: {error}; use --download or import a local copy"
                ) from error
        else:
            return destination
    destination.parent.mkdir(parents=True, exist_ok=True)
    scratch = None
    try:
        with NamedTemporaryFile(dir=destination.parent, prefix=f".{destination.name}-", delete=False) as stream:
            scratch = Path(stream.name)
            if local is None:
                fetch(pin, stream)
            else:
                with regular_file(local, pin.max_bytes) as source:
                    copy_bounded(source, stream, pin.max_bytes)
            validate_container(stream, destination.name)
        scratch.replace(destination)
    except (OSError, ValueError, http.client.HTTPException) as error:
        origin = pin.url if local is None else local
        raise ValueError(f"cannot acquire {destination.name} from {origin}: {error}") from error
    finally:
        if scratch is not None:
            scratch.unlink(missing_ok=True)
    return destination


def prepare_assets(project: Project, cache_path: Path, *, download: bool = False,
                   apk: Path | None = None, source: Path | None = None) -> Assets:
    """Reuse, import or explicitly download assets without unpacking either archive."""
    if not PROJECT_ID.fullmatch(project.id):
        raise ValueError("invalid project cache ID")
    directory = Path(cache_path).resolve() / project.id
    if directory.is_symlink():
        raise ValueError(f"project cache must not be a symlink: {directory}")
    return Assets(prepare_asset(directory / "input.apk", project.apk, apk, download),
                  prepare_asset(directory / "source.tar.gz", project.source, source, download))
