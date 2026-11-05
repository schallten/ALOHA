import os

import fluxer

bot = fluxer.Bot(command_prefix="!", intents=fluxer.Intents.default())


@bot.event
async def on_ready():
    print(f"ALOHA is online as {bot.user.username}")


@bot.command()
async def ping(ctx):
    await ctx.reply("Pong!")


if __name__ == "__main__":
    token = os.environ.get("FLUXER_TOKEN")
    if not token:
        raise SystemExit("set FLUXER_TOKEN env var")
    bot.run(token)
