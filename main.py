import os
from collections import deque

from dotenv import load_dotenv
import fluxer

from constants import (
    MAX_ACTIVE_PAIRS,
    MAX_QUEUE_SIZE,
    MIN_SEND_INTERVAL,
    COLOR_CONNECTED,
    COLOR_DISCONNECTED,
    COLOR_INFO,
)
from state import CallManager, pair_waiting_users
from pacer import Pacer
from sanitize import relay_text

load_dotenv()

# fluxer.py `rework` branch (0.5.0-alpha, not on PyPI yet): presence is a
# custom status, not a rich activity -- `bot.change_presence(activity=...)`
# sends {"custom_status": {"text": ...}}, which is what Fluxer renders.
# Use fluxer.Client, NOT fluxer.Bot: Bot (ext.commands) auto-processes
# `u!...` messages as commands and logs CommandNotFound for our manually-
# handled !userphone/!hangup. Client has change_presence() too, but no
# command processing, so on_message below is the only handler.
bot = fluxer.Client()

call_manager = CallManager()
pacer = Pacer(MIN_SEND_INTERVAL)


@bot.event
async def on_ready():
    print(f"ALOHA is online as {bot.user.username}")
    await bot.change_presence(activity="u!userphone - tell your friends about me")


@bot.event
async def on_resumed():
    await bot.change_presence(activity="u!userphone - tell your friends about me")


async def send_to_channel_http(channel_id, content=None, embed=None):
    try:
        await pacer.wait(channel_id)
        if embed is not None:
            # pass embed
            await bot._http.send_message(channel_id, embed=embed)
            return
        if content is not None:
            await bot._http.send_message(channel_id, content=content)
            return
    except Exception as e:
        print(f"send error {e}")


async def send_connected_embed(channel_id, partner_id):
    embed = fluxer.Embed(
        title="Connection Found ☎️",
        description=f"You've been connected with <@{partner_id}>!\n\nSay hi, be kind, and please keep it friendly. 💚\nWhen you're done, type `!hangup`.",
        color=COLOR_CONNECTED,
    )
    await send_to_channel_http(channel_id, embed=embed)


async def announce_hangup(partner_id, partner_channel, hanger_id):
    embed = fluxer.Embed(
        title="Call Ended",
        description=f"<@{hanger_id}> hung up the call.\n\nHope it was a good chat! If you'd like to talk to someone new, type `!userphone` to join the queue again.",
        color=COLOR_DISCONNECTED,
    )
    await send_to_channel_http(partner_channel, embed=embed)


@bot.event
async def on_message(message):
    try:
        if message.author == bot.user:
            return
        content = (message.content or "").strip()
        # Presence advertises `u!userphone`, handlers below check `!cmd` --
        # accept both spellings.
        if content.startswith("u!"):
            content = content[1:]
        user_id = str(message.author.id)

        if not content.startswith('!'):
            partner_dest = None
            for call in call_manager.active_calls:
                pd = call.partner_destination(user_id)
                if pd is not None:
                    partner_dest = pd
                    break
            if partner_dest is not None:
                partner_channel, partner_guild = partner_dest
                username = getattr(message.author, 'username', None) or getattr(message.author, 'name', str(user_id))
                relay_msg = relay_text(username, content)
                try:
                    await send_to_channel_http(partner_channel, content=relay_msg)
                except Exception as e:
                    print(f"relay error {e}")
            return

        if content == "!ping":
            embed = fluxer.Embed(title="Pong", color=0x5865F2)
            try:
                await message.reply(embed=embed)
            except Exception as e:
                print(f"send error {e}")
            return

        if content == "!userphone":
            new_pairs = []
            title = ""
            description = ""
            color = COLOR_INFO
            image_url = None

            already_active = False
            for c in call_manager.active_calls:
                if c.contains(user_id):
                    already_active = True
                    break
            already_queued = False
            for q in call_manager.user_queue:
                if q[0] == user_id:
                    already_queued = True
                    break

            if already_queued or already_active:
                title = "Already in Queue"
                description = f"You're already in the queue, <@{message.author.id}>!\nPlease wait for your turn."
                color = 0xF1C40F
                image_url = "https://media1.tenor.com/m/v52lOlHNtjAAAAAd/ronaldo-cheeky-boy.gif"
            elif len(call_manager.active_calls) >= MAX_ACTIVE_PAIRS:
                title = "All Lines Busy 📵"
                description = f"All **{MAX_ACTIVE_PAIRS}** call slots are in use right now, <@{message.author.id}>.\nSomeone will hang up soon — please try again in a little while. 🙏"
                color = COLOR_DISCONNECTED
                image_url = "https://media1.tenor.com/m/v52lOlHNtjAAAAAd/ronaldo-cheeky-boy.gif"
            elif len(call_manager.user_queue) >= MAX_QUEUE_SIZE:
                title = "Queue Full 🚦"
                description = f"The waiting queue is full (**{MAX_QUEUE_SIZE}** people), <@{message.author.id}>.\nPlease try again in a little while. 🙏"
                color = COLOR_DISCONNECTED
                image_url = "https://media1.tenor.com/m/v52lOlHNtjAAAAAd/ronaldo-cheeky-boy.gif"
            else:
                guild_id = str(message.guild_id) if message.guild_id is not None else ""
                call_manager.user_queue.append((user_id, str(message.channel_id), guild_id))
                new_pairs = pair_waiting_users(call_manager, MAX_ACTIVE_PAIRS)
                paired = None
                for p in new_pairs:
                    if p.contains(user_id):
                        paired = p
                        break
                if paired is not None:
                    partner = paired.user2 if paired.user1 == user_id else paired.user1
                    title = "Connection Found ☎️"
                    description = f"You've been connected with <@{partner}>!\n\nSay hi, be kind, and please keep it friendly. 💚\nWhen you're done, type `!hangup`."
                    color = COLOR_CONNECTED
                else:
                    title = "Successfully Joined"
                    description = f"You have successfully joined the queue, <@{message.author.id}>!\nPlease wait while we establish your connection."
                    color = COLOR_INFO
                image_url = "https://media1.tenor.com/m/47EjvEixiNUAAAAd/cop-detective.gif"

            embed = fluxer.Embed(title=title, description=description, color=color)
            if image_url:
                embed.set_image(url=image_url)
            try:
                await message.reply(embed=embed)
            except Exception as e:
                print(f"send error {e}")

            own_channel = str(message.channel_id)
            for call in new_pairs:
                if str(call.channel1) != own_channel:
                    await send_connected_embed(call.channel1, call.user2)
                if str(call.channel2) != own_channel:
                    await send_connected_embed(call.channel2, call.user1)
            return

        if content == "!hangup":
            ended_call = None
            for i, c in enumerate(call_manager.active_calls):
                if c.contains(user_id):
                    ended_call = call_manager.active_calls.pop(i)
                    break
            new_queue = []
            for q in call_manager.user_queue:
                if q[0] != user_id:
                    new_queue.append(q)
            call_manager.user_queue = deque(new_queue)

            new_pairs = []
            if ended_call is not None:
                new_pairs = pair_waiting_users(call_manager, MAX_ACTIVE_PAIRS)
                pd = ended_call.partner_destination(user_id)
                if pd is not None:
                    partner_channel, _ = pd
                    partner_id = ended_call.user2 if ended_call.user1 == user_id else ended_call.user1
                    await announce_hangup(partner_id, partner_channel, user_id)
                embed = fluxer.Embed(title="Hanging up", description="You've left the call. Thanks for being kind! 💚", color=COLOR_INFO)
                embed.set_image(url="https://media1.tenor.com/m/rYE88J8ArOUAAAAd/phone-anime.gif")
            else:
                embed = fluxer.Embed(title="you weren't in a call", color=COLOR_INFO)
                embed.set_image(url="https://media1.tenor.com/m/rYE88J8ArOUAAAAd/phone-anime.gif")

            try:
                await message.reply(embed=embed)
            except Exception as e:
                print(f"send error {e}")

            for call in new_pairs:
                await send_connected_embed(call.channel1, call.user2)
                await send_connected_embed(call.channel2, call.user1)
            return

    except Exception as e:
        print(f"error {e}")


def main():
    token = os.environ.get("FLUXER_TOKEN_TEST") or os.environ.get("FLUXER_TOKEN")
    if not token:
        raise SystemExit("set FLUXER_TOKEN env var")
    bot.run(token)


if __name__ == "__main__":
    main()
