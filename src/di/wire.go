//+build wireinject

package di

import (
	"fmt"

	"github.com/google/wire"
)

type Bot struct{}

func (b *Bot) Start() {
	fmt.Println("Hey")
}

func InitializeBot() (Bot, error) {
	wire.Build(NewBot)
	return Bot{}, nil
}

func NewBot() Bot {
	return Bot{}
}
