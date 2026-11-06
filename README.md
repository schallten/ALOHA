# ALOHA

[INVITE](https://fluxer.com/oauth2/authorize?client_id=1565326521504112643&scope=bot&permissions=274877910016) | [SUPPORT](https://fluxer.gg/G7H6l5wu)

Name inspired by the ALOHA network protocol, connecting independent users through
a shared medium, one connection at a time. A [fluxer.app](https://fluxer.app) bot.

ALOHA pairs up strangers so they can talk to each other from their own channels,
even when those channels are in different servers. What you type is delivered to
your partner's channel, and theirs to yours, until one of you hangs up.

## Commands

| Command | What it does |
| --- | --- |
| `u!userphone` | Join the queue. If someone is already waiting, you get paired with them right away. |
| `u!hangup` | End the call. Your partner is told you hung up. |
| `u!ping` | Health check. |
| `u!help` | Show the command list. |

Any message that does not start with `u!` gets relayed to your partner as
`**yourname:** your message`.

## Notes

- Pairing is cross-server: your partner can be in a completely different guild.
- A pair is bound to the channels you both used, so quiet channels work best.
- Up to **100 calls** at once, and up to **200 people** waiting in the queue.
- The bot sends at most one message every **500 ms** per channel.
- Mentions (`@everyone`, `@here`, user and role pings) are disarmed in relayed
  messages, so nobody can ping a server through the bot.
- Links, images and formatting are stripped: anything link-like becomes
  `[link removed]`, and `*`, `_`, `` ` ``, `~`, `|` can no longer render, so
  nobody can push an embedded GIF or a tracking link into your channel.
- Messages longer than the platform limit are trimmed with `...`.

See [PRIVACY.md](PRIVACY.md) for what the bot does with your data.

## Implementation

The bot is written in Python using
[`fluxer.py`](https://github.com/Fluxer-py/fluxer.py). The previous Rust
implementation (and its Dockerfile) has been moved to `legacy/` for reference.
Core behavior from the Rust version (queueing, pairing, relaying, sanitization,
rate limiting, and caps) has been ported to Python.

**Not ready for big servers yet.**

## Running

Set your token (e.g. `FLUXER_TOKEN` or `FLUXER_TOKEN_TEST`) in the environment or `.env`, then:

```bash
python main.py
```

Or with Docker:

```bash
docker build -t aloha .
docker run -e FLUXER_TOKEN=your-token aloha
```
