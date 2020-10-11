package main

import (
	"fmt"
	"os"

	"github.com/dfontana/disbot/di"
)

func main() {
	e, err := di.InitializeBot()
	if err != nil {
		fmt.Printf("Failed to create event: %s\n", err)
		os.Exit(2)
	}
	err = e.Start()
	if err != nil {
		fmt.Printf("Failed to start bot: %s\n", err)
		os.Exit(2)
	}
}
