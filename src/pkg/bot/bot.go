package bot

import "fmt"

// Bot client
type Bot struct {
	config Config
}

// NewBot builder
func NewBot(config Config) Bot {
	return Bot{config}
}

// Start the bot lifecycle
func (b *Bot) Start() {
	fmt.Println(b.config.getKey())
}
