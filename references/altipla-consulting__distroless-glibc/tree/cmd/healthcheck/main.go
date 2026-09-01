package main

import (
	"context"
	"flag"
	"fmt"
	"net/http"
	"os"
	"time"
)

func main() {
	var flagURL, flagFile string
	var flagTimeout time.Duration
	flag.StringVar(&flagURL, "url", "", "URL to check")
	flag.StringVar(&flagFile, "file", "", "File path to check")
	flag.DurationVar(&flagTimeout, "timeout", time.Second*5, "Timeout for the request")
	flag.Parse()

	if flagURL != "" {
		ctx, cancel := context.WithTimeout(context.Background(), flagTimeout)
		defer cancel()
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, flagURL, nil)
		if err != nil {
			fmt.Printf("failed to prepare request: %s", err)
			os.Exit(1)
		}
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			fmt.Printf("failed to send healthcheck request: %s", err)
			os.Exit(1)
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusOK {
			fmt.Printf("unexpected status %v", resp.Status)
			os.Exit(1)
		}
	}

	if flagFile != "" {
		if _, err := os.Stat(flagFile); err != nil {
			fmt.Printf("file %s does not exist", flagFile)
			os.Exit(1)
		}
	}

	fmt.Printf("healthy")
}
