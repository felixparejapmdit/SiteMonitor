"""Extract a named JS function's full text (string/template/comment aware)."""
import re, os, sys, json

SERVICE5 = os.environ.get("SERVICE5_DIR") or os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "01_Rust_WC_DB"))
ROOT = os.path.join(SERVICE5, "js")
BS = chr(92)


def skip_string(t, j, q):
    j += 1
    while True:
        c = t[j]
        if c == BS:
            j += 2
            continue
        if c == q:
            return j
        if q == '`' and c == '$' and t[j + 1] == '{':
            j = skip_block(t, j + 1)
        j += 1


def is_regex_start(t, j):
    k = j - 1
    while k >= 0 and t[k] in ' \t\r\n':
        k -= 1
    if k < 0 or t[k] in '(,=:[!&|?{};+-*%<>~^':
        return True
    return re.search(r'(?:return|typeof|case|in|of)$', t[max(0, k - 6):k + 1]) is not None


def skip_regex(t, j):
    j += 1
    cls = False
    while True:
        c = t[j]
        if c == BS:
            j += 2
            continue
        if c == '[':
            cls = True
        elif c == ']':
            cls = False
        elif c == '/' and not cls:
            return j
        elif c == '\n':
            return j
        j += 1


def skip_block(t, i):
    """t[i] == '{'; return index of matching '}'."""
    d = 0
    j = i
    while True:
        c = t[j]
        if c in '"\'`':
            j = skip_string(t, j, c)
        elif c == '/' and t[j + 1] == '/':
            j = t.index('\n', j)
            continue
        elif c == '/' and t[j + 1] == '*':
            j = t.index('*/', j) + 1
        elif c == '/' and is_regex_start(t, j):
            j = skip_regex(t, j)
        elif c == '{':
            d += 1
        elif c == '}':
            d -= 1
            if d == 0:
                return j
        j += 1


def extract(t, fn):
    pats = [r'window\.' + re.escape(fn) + r'\s*=\s*(?:async\s+)?function\s*\w*\s*\(',
            r'function\s+' + re.escape(fn) + r'\s*\(',
            r'(?<![\w.])' + re.escape(fn) + r'\s*[:=]\s*(?:async\s+)?function\s*\w*\s*\(',
            r'(?<![\w.])' + re.escape(fn) + r'\s*[:=]\s*(?:async\s*)?\([^)]*\)\s*=>\s*\{',
            r'^\s*(?:async\s+)?' + re.escape(fn) + r'\s*\([^)]*\)\s*\{']
    for p in pats:
        m = re.search(p, t, re.M)
        if m:
            break
    else:
        return None, 0
    k = m.start() + m.group(0).index('(') if '(' in m.group(0) else m.end()
    d = 0
    j = k
    while True:
        if t[j] == '(':
            d += 1
        elif t[j] == ')':
            d -= 1
            if d == 0:
                break
        elif t[j] in '"\'`':
            j = skip_string(t, j, t[j])
        j += 1
    i = t.index('{', j)
    end = skip_block(t, i)
    start = m.start()
    if t[max(0, start - 6):start] == 'async ':
        start -= 6
    while start > 0 and t[start - 1] in ' \t':
        start -= 1
    return t[start:end + 1], t[:m.start()].count('\n') + 1


def extract_decl(t, name):
    """`const NAME = <expr>;` (top level or indented), verbatim."""
    m = re.search(r'^[ \t]*(?:(?:const|let|var)\s+|window\.)' + re.escape(name) + r'\s*=', t, re.M)
    if not m:
        return None
    j = m.end()
    d = 0
    while True:
        c = t[j]
        if c in '"\'`':
            j = skip_string(t, j, c)
        elif c == '/' and t[j + 1] == '/':
            j = t.index('\n', j)
            continue
        elif c == '/' and t[j + 1] == '*':
            j = t.index('*/', j) + 1
        elif c in '([{':
            d += 1
        elif c in ')]}':
            d -= 1
        elif c == ';' and d == 0:
            return t[m.start():j + 1]
        elif c == '\n' and d == 0 and t[m.end():j].strip():
            # statement without semicolon: ends at newline when next line isn't a continuation
            nxt = t[j + 1:j + 200].lstrip()
            if not nxt[:1] in '.?:+-*/|&,':
                return t[m.start():j] + ';'
        j += 1


def get(file, name):
    t = open(os.path.join(ROOT, file), encoding='utf-8').read()
    body, _ = extract(t, name)
    if body is None:
        body = extract_decl(t, name)
    if body is None:
        raise SystemExit(f'not found: {file} {name}')
    return body


B = set("if for while switch return function catch typeof new Array Object String Number Math JSON Date Promise Set Map parseInt parseFloat isNaN encodeURIComponent setTimeout clearTimeout console window document Boolean RegExp Error alert confirm".split())

if __name__ == '__main__':
    spec = json.loads(sys.argv[1])
    for f, fns in spec.items():
        t = open(os.path.join(ROOT, f), encoding='utf-8').read()
        for fn in fns:
            body, line = extract(t, fn)
            if not body:
                print(f, fn, 'MISSING')
                continue
            local = set(re.findall(r'(?:function|const|let|var)\s+([A-Za-z_$][\w$]*)', body)) | set(re.findall(r'[(,]\s*([A-Za-z_$][\w$]*)\s*(?=[,)])', body))
            calls = set(re.findall(r'(?<![.\w$])([A-Za-z_$][\w$]*)\s*\(', body)) - B - local
            glob = set(re.findall(r'(?<![.\w$])([A-Za-z_$][\w$]*(?:Data|_NAMES|Map|List|State|Groups))\b', body)) - local
            print(f"{f}:{line} {fn} [{body.count(chr(10))} lines] calls={sorted(calls)} globals={sorted(glob)}")
