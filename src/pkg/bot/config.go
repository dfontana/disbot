package bot

import (
	"errors"
	"os"
)

// Config for Discord API
type Config struct {
	apiKey string
	apiID  string
}

func (conf *Config) getKey() string {
	return conf.apiKey
}

func (conf *Config) getID() string {
	return conf.apiID
}

// NewConfig builder
func NewConfig() (Config, error) {
	key := os.Getenv("API_KEY")
	if key == "" {
		return Config{}, errors.New("Missing API_KEY on this host")
	}
	id := os.Getenv("API_ID")
	if id == "" {
		return Config{}, errors.New("Missing API_ID on this host")
	}
	return Config{key, id}, nil
}
