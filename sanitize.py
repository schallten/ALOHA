from constants import INVISIBLE, LINK_MARKERS, RELAY_CONTENT_LIMIT


def neutralize_mentions(text):
    out = []
    i = 0
    chars = text
    n = len(chars)
    j = 0
    # easier to iterate by index
    s = chars
    i = 0
    res = []
    while i < len(s):
        c = s[i]
        res.append(c)
        if c == '<' and i + 1 < len(s) and s[i + 1] == '@':
            res.append(INVISIBLE)
        if c == '@':
            # peek next 8
            nxt = s[i + 1 : i + 1 + 8].lower()
            if nxt.startswith('everyone') or nxt.startswith('here'):
                res.append(INVISIBLE)
        i += 1
    return ''.join(res)


def remove_links(text):
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c.isspace():
            out.append(c)
            i += 1
            continue
        start = i
        while i < n and not text[i].isspace():
            i += 1
        word = text[start:i]
        ws_start = i
        while i < n and text[i].isspace():
            i += 1
        trailing_ws = text[ws_start:i]
        lower = word.lower()
        has_link = False
        for marker in LINK_MARKERS:
            if marker in lower:
                has_link = True
                break
        if has_link:
            out.append('[link removed]')
            out.append(trailing_ws)
        else:
            out.append(word)
            out.append(trailing_ws)
    return ''.join(out)


def remove_markdown_links(text):
    text = text.replace('![', '[')
    text = text.replace('](', '] (')
    return text


def defuse_markdown(text):
    out = []
    for c in text:
        out.append(c)
        if c == '*' or c == '_' or c == '`' or c == '~' or c == '|':
            out.append(INVISIBLE)
    return ''.join(out)


def sanitize_content(text):
    text = remove_links(text)
    text = remove_markdown_links(text)
    text = defuse_markdown(text)
    return text


def relay_text(username, content):
    full = neutralize_mentions('**' + username + ':** ' + sanitize_content(content))
    # Rust counts bytes (full.len() in Rust is byte length), not chars, and
    # never slices a multi-byte char in half. Mirror that here.
    if len(full.encode('utf-8')) <= RELAY_CONTENT_LIMIT:
        return full
    keep_under = RELAY_CONTENT_LIMIT - 3
    out_list = []
    used = 0
    for c in full:
        size = len(c.encode('utf-8'))
        if used + size > keep_under:
            break
        out_list.append(c)
        used += size
    return ''.join(out_list) + '...'
