package service

import (
	"context"
	"log/slog"
	"time"
)

type Scheduler struct {
	discovery *DiscoveryService
	interval  time.Duration
	stopCh    chan struct{}
}

func NewScheduler(discovery *DiscoveryService, interval time.Duration) *Scheduler {
	return &Scheduler{
		discovery: discovery,
		interval:  interval,
		stopCh:    make(chan struct{}),
	}
}

func (s *Scheduler) Start() {
	go func() {
		slog.Info("scheduler started", "interval", s.interval)

		// Run once after a short delay to let the server stabilize
		select {
		case <-time.After(30 * time.Second):
			s.runDiscovery()
		case <-s.stopCh:
			return
		}

		ticker := time.NewTicker(s.interval)
		defer ticker.Stop()

		for {
			select {
			case <-ticker.C:
				s.runDiscovery()
			case <-s.stopCh:
				slog.Info("scheduler stopped")
				return
			}
		}
	}()
}

func (s *Scheduler) Stop() {
	close(s.stopCh)
}

func (s *Scheduler) runDiscovery() {
	slog.Info("scheduler: triggering auto-discovery")
	ctx := context.Background()
	job, err := s.discovery.Discover(ctx)
	if err != nil {
		slog.Warn("scheduler: discovery failed to start", "error", err)
		return
	}
	slog.Info("scheduler: discovery job started", "job_id", job.ID)
}
