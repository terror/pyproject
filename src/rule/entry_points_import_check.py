import importlib
import inspect
import json
import os
import sys

data = json.load(sys.stdin)

cwd = os.getcwd()


def dedupe_paths(paths):
  seen = set()

  for path in paths:
    canonical = os.path.abspath(path) if path else cwd

    if canonical in seen:
      continue

    seen.add(canonical)

    yield path


project_paths = []

src_path = os.path.join(cwd, 'src')

if os.path.isdir(src_path):
  project_paths.append(src_path)

base_path = list(dedupe_paths(project_paths + list(sys.path)))

isolated_path = [
  path for path in base_path
  if path and os.path.abspath(path) != cwd
]


def try_import(path, module, qualname):
  sys.path[:] = path

  try:
    module_obj = importlib.import_module(module)
  except Exception as exc:  # pragma: no cover - surfaced to Rust caller
    return False, f"{type(exc).__name__}: {exc}"

  target = module_obj

  if qualname:
    for part in qualname.split('.'):
      try:
        target = inspect.getattr_static(target, part)
      except AttributeError:
        return False, f"missing attribute {part}"
      except Exception as exc:  # pragma: no cover - surfaced to Rust caller
        return False, f"{type(exc).__name__}: {exc}"

  return True, None


results = []

for entry in data:
  ok, isolated_error = try_import(
    isolated_path,
    entry['module'],
    entry.get('qualname'),
  )

  if ok:
    results.append({'index': entry['index'], 'status': 'ok'})
    continue

  ok, default_error = try_import(
    base_path,
    entry['module'],
    entry.get('qualname'),
  )

  if ok:
    results.append({
      'index': entry['index'],
      'status': 'needs-cwd',
      'isolated_error': isolated_error,
    })
    continue

  results.append({
    'index': entry['index'],
    'status': 'error',
    'isolated_error': isolated_error,
    'error': default_error,
  })

json.dump(results, sys.stdout)
