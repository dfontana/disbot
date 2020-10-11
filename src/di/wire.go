//+build wireinject

package di

import (
	"github.com/dfontana/disbot/pkg/bot"
	"github.com/google/wire"
)

// InitializeBot builds the bot for running
func InitializeBot() (bot.Bot, error) {
	wire.Build(bot.NewBot, bot.NewConfig, bot.NewEmojiCache)
	return bot.Bot{}, nil
}
