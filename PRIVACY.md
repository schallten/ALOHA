# ALOHA — Privacy Notice

_Last updated: 4 November 2026_

ALOHA is a chat-relay bot for [fluxer.app](https://fluxer.app). It pairs two
people who are waiting, then forwards what each one types to the other's channel.
This notice describes exactly what the bot does with data. It is a short
description of the software, not legal advice.

## What the bot processes

- **IDs.** Your user ID, the channel ID and the guild ID where you ran
  `!userphone`. These are needed to know who to pair and where to deliver.
- **Message text.** Messages you send while a call is active are read so they can
  be forwarded to your partner's channel.
- **Nothing else.** No email, no password, no profile data, no attachments
  (files, stickers and images are not forwarded).

## Where it is stored

- **In memory only.** The IDs above live in the running bot process while you are
  queued or in a call. There is no database and no file storage.
- **Cleared automatically.** The entry is removed when either person runs
  `!hangup`, and everything is lost when the bot restarts.
- **Message text is not stored.** It passes through memory on its way to your
  partner. The copy you see in their channel is a normal message in that channel,
  stored by the platform like any other message — not by the bot.

## What the bot does not do

- No analytics, telemetry, profiling or advertising.
- No cookies and no tracking links — link-like content is removed from relayed
  messages before they are sent.
- No third-party services. The only outbound connections are to the fluxer API.
- No IP address collection, and no logging of message content.
- Nothing is sold, shared or transferred to anyone.

## Logs

The bot writes operational logs to standard output: start-up, connection errors,
failed relays, and technical IDs involved in those events. **Message content is
never written to logs.**

If you self-host, those logs exist wherever you send them (for example Docker's
log file) and are kept or rotated according to your own setup.

## Retention and deletion

- Queue and call data: deleted on hangup, or on restart — whichever comes first.
- Logs: retained by whoever runs the bot, typically only as long as needed to
  debug a problem.
- Relayed messages: the bot has no memory of them and no command to delete them.
  Deleting your original message does **not** delete the copy delivered to your
  partner's channel; use the platform's normal message tools for that.

## Who can see your messages

Your partner sees the messages relayed to their channel. The person operating the
bot could read them if they inspected traffic, and the platform stores them as
normal messages under its own privacy policy. Treat ALOHA as a conversation with
a stranger, not as a private or encrypted channel.

## Change of hands

If the bot is transferred to another operator or shut down, the in-memory state
goes with it: nothing is handed over, because nothing is written down.

## Contact and changes

This notice may be updated when the bot changes. If something here becomes
inaccurate, the current version in the project repository is the one that applies.
For questions, contact the operator of the instance you are using.
