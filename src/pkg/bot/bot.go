package bot

import (
	"errors"
	"fmt"
	"os"
	"os/signal"
	"syscall"

	dg "github.com/bwmarrin/discordgo"
)

// Bot client
type Bot struct {
	dd      *dg.Session
	emcache EmojiCache
	users   map[string]bool
}

// NewBot builder
func NewBot(config Config, emcache EmojiCache) (Bot, error) {
	dd, err := dg.New("Bot " + config.getKey())
	if err != nil {
		return Bot{}, errors.New("Failed build discord client")
	}
	mapped := make(map[string]bool)
	for _, user := range config.getUsers() {
		mapped[user] = true
	}
	return Bot{dd, emcache, mapped}, nil
}

// Start the bot lifecycle
func (b *Bot) Start() error {
	b.dd.Identify.Intents = dg.MakeIntent(dg.IntentsGuildMessages)
	b.dd.AddHandler(b.onMessageCreate)

	fmt.Println("Connecting to Discord...")
	if err := b.dd.Open(); err != nil {
		return err
	}

	fmt.Println("Listening...")
	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt, os.Kill)
	<-sc

	b.dd.Close()
	return nil
}

func (b *Bot) onMessageCreate(s *dg.Session, m *dg.MessageCreate) {
	if m.Author.ID == s.State.User.ID {
		// This message was from you
		return
	}
	if containsUser(m.Mentions, b.users) {
		emoji, err := b.emcache.get(s, m)
		if err != nil {
			fmt.Printf("Failed to pull emojis: %s, %s\n", m.GuildID, err)
			s.ChannelMessageSend(m.ChannelID, "You taketh my shrug, you taketh me :(")
			return
		}
		s.MessageReactionAdd(m.ChannelID, m.ID, emoji.react)
		s.ChannelMessageSend(m.ChannelID, emoji.message)
	}
}

func containsUser(mentions []*dg.User, names map[string]bool) bool {
	for _, user := range mentions {
		if _, ok := names[user.Username]; ok {
			return true
		}
	}
	return false
}
