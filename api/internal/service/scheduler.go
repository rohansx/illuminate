package service

import (
	"context"
	"log/slog"
	"time"
)

type Scheduler struct {
	discovery      *DiscoveryService
	contribService *ContributionService
	interval       time.Duration
	stopCh         chan struct{}
}

func NewScheduler(discovery *DiscoveryService, contribService *ContributionService, interval time.Duration) *Scheduler {
	return &Scheduler{
		discovery:      discovery,
		contribService: contribService,
		interval:       interval,
		stopCh:         make(chan struct{}),
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

		// Contribution sync runs every other tick (12h if interval is 6h)
		contribTick := 0

		for {
			select {
			case <-ticker.C:
				s.runDiscovery()
				contribTick++
				if contribTick%2 == 0 {
					s.runContributionSync()
				}
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

func (s *Scheduler) runContributionSync() {
	slog.Info("scheduler: triggering contribution sync")
	ctx := context.Background()
	job, err := s.contribService.SyncAll(ctx)
	if err != nil {
		slog.Warn("scheduler: contribution sync failed to start", "error", err)
		return
	}
	slog.Info("scheduler: contribution sync job started", "job_id", job.ID)
}
