import subprocess, os, re
env = {**os.environ, 'CARGO_BUILD_RUSTC_WRAPPER': ''}
r = subprocess.run(['cargo', '+nightly', 'rustc', '-p', 'specta-tests', '--test', 'test', '--quiet', '--',
                    '-Zunpretty=expanded'], capture_output=True, text=True, env=env)
out = r.stdout
print('rc', r.returncode, 'len', len(out))
i = out.find('impl<T> specta::Type for NoInferredBound')
if i < 0:
    i = out.find('NoInferredBound')
print(out[i-200:i+2600])