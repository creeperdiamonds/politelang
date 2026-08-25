# Discord bots, in PoliteLang

Two working bots, written entirely in English sentences.

| | |
| --- | --- |
| **`hello-bot.polite`** | listens for people talking, and answers |
| **`club-bot.polite`** | slash commands, buttons, cards, reactions, roles, and people arriving |

`hello-bot.polite` is the one to read first. It has no
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

## Everything else it can do

`club-bot.polite` uses all of this. It is still a loop — it just listens for more than talking.

**Listening for anything, not only messages**

```
listen for whatever happens next     wait for anything the bot can see
watch people joining and leaving     say this BEFORE logging in, if you want arrivals

it was a message      it was a command      it was a button
it was a reaction     somebody joined       somebody left
```

**Slash commands**

```
offer the command {word} for {description}
offer the command {word} taking {things} for {description}
the command they used
what they gave for {which}
reply quietly with {text}            only the person who asked sees it
```

Commands are offered to every server the bot is in, so they appear straight away rather than in an
hour. Discord waits **three seconds** for an answer to a command or a button, so answer before doing
anything slow.

**Cards, buttons and reactions**

```
post a card titled {title} saying {words}
post a card made of {details}        a lookup: title, words, colour, footer, image, link
present {text} with buttons {labels} up to five; the words on a button are also its name
the button they pressed
react with {emoji}
the emoji they used
start typing
```

**Saying things elsewhere, and taking them back**

```
announce {text} in the channel called {channel}
correct what I said to {text}
delete what they said
```

**Roles, and keeping order**

```
give them the role called {role}
remove the role called {role} from them
they have the role called {role}
the roles they have
let their nickname be {nick}
quieten them for {minutes} minutes   Discord allows up to 28 days
remove them from the server          a kick; they can be invited back
ban them from the server
```

**Who and where**

```
their id                             never changes, unlike a name
the people in the server
the channels in the server
```

### One thing worth knowing about the moderation words

They all act on **whoever the last thing that happened was about**. There is no way to reach across
and act on somebody else. That is deliberate: a bot that can only act on the person in front of it
is a much harder bot to turn into a weapon. So `quieten them` after somebody says something is
straightforward, and `quieten whoever I name` is not something this vocabulary can express.

### For `somebody joined` to work

`watch people joining and leaving` must come **before** `log in to discord`, and *Server Members
Intent* must be on in the developer portal. Without the second, Discord refuses the whole
connection — so the intent is only ever asked for when a program says it wants it, and a bot that
never mentions arrivals never pays that price.

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
