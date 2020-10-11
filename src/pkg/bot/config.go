package bot

import (
	"errors"
	"os"
)

// Config for Discord API
type Config struct {
	apiKey string
}

func (conf *Config) getKey() string {
	return conf.apiKey
}

// NewConfig builder
func NewConfig() (Config, error) {
	key := os.Getenv("API_KEY")
	if key == "" {
		return Config{}, errors.New("Missing API_KEY on this host")
	}
	return Config{key}, nil
}
