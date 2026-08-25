# A Discord bot, in PoliteLang

`hello-bot.polite` is a working Discord bot written entirely in English sentences. It has no
handlers and nothing to register — it is a loop that listens, looks at what was said, and answers.

```polite
please keep going forever:
    listen for the next message
    check if what they said is "!ping":
        reply with "Pong."
    thanks
thank you for repeating
```

## Getting it running

**1. Make a bot and get its token.** At <https://discord.com/developers/applications>: *New
Application* → *Bot* → *Reset Token* → copy it. The token is a password for your bot. Anybody who
has it can be your bot, so it never goes in a file you show people, and if it ever leaks, press
*Reset Token* and the old one stops working.

**2. Turn on Message Content Intent.** Same page, under *Bot* → *Privileged Gateway Intents*. Without
it Discord sends your bot every message with the words removed, and `what they said` is always
empty. This catches everybody once.

**3. Invite it to a server.** *OAuth2* → *URL Generator* → tick **bot**, then under permissions tick
*Send Messages*, *Read Message History* and *View Channels*. Open the URL it builds and pick your
server. You need *Manage Server* on that server, so use one of your own.

**4. Build and run it.**

```
polite build examples/discord/hello-bot.polite
cd examples/discord
npm install discord.js
node hello-bot.mjs
```

**5. Give it the token.** The easiest way is a `.env` file in this folder. Copy the example:

```
cp .env.example .env
```

and put your token in it:

```
DISCORD_TOKEN=your token here
```

`.env` is in `.gitignore`, so it cannot be committed by accident. `.env.example` is committed and
holds nothing but a reminder.

If you would rather hand it over as you start the bot, that works too, and beats the file if both
are set. PowerShell:

```powershell
$env:DISCORD_TOKEN = "your token here"
node hello-bot.mjs
```

Bash:

```bash
DISCORD_TOKEN="your token here" node hello-bot.mjs
```

Then say `hello` in a channel the bot can see.

## What it knows

| you say | it does |
| --- | --- |
| `hello`, `hi`, `hey`, `good morning`, `good evening` | says hello back, by your name |
| `!ping` | says `Pong.` |
| `!where` | names the channel and the server |
| `!say <words>` | says those words |
| `!count <words>` | counts the words |
| `!help` | lists all of this |
| `!goodbye` | says goodbye and stops |

## The words it is built from

```
log in to discord with {token}      join Discord as the bot the token belongs to
listen for the next message         wait, doing nothing, until somebody speaks
reply with {text}                   answer the message just heard, attached to it
send {text}                         say something in the same channel, on its own
let my status be {text}             the line under the bot's name in the member list

what they said                      the words of the message
their name                          who said it, as shown on the server
they are a bot                      whether it was a bot rather than a person
the channel it came from            the name of the channel
the server it came from             the name of the server

the secret called {which}           read from the surroundings, or from .env in this folder
```

`polite explain "reply with"` says what any of them mean, in PoliteLang and in English.

## Two things it does for you

**It never hears itself.** A bot that hears its own messages answers itself, then answers that, and
keeps going until somebody pulls the plug. Messages from the bot itself are never delivered.
Messages from *other* bots are, so that `they are a bot` is a question worth asking.

**One bad message costs one message.** Everything to do with a single message sits inside `try to`,
so a reply the bot was not allowed to send does not end the evening — it prints why and waits for
the next one.

## When something goes wrong

| what it says | what to do |
| --- | --- |
| `there is no secret called DISCORD_TOKEN here` | put it in `.env`, or hand it over as you start it |
| `Discord would not take that token` | wrong or expired — *Reset Token* and use the new one |
| `discord.js is not installed here` | `npm install discord.js` in this folder |
| it connects but `what they said` is always empty | Message Content Intent is off — step 2 |
| `I am not allowed to reply there` | the bot lacks *Send Messages* in that channel |
