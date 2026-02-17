package service

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
)

type JobManager struct {
	mu   sync.RWMutex
	jobs map[string]*model.JobStatus
}

func NewJobManager() *JobManager {
	return &JobManager{
		jobs: make(map[string]*model.JobStatus),
	}
}

func (m *JobManager) StartJob(jobType string, fn func(ctx context.Context, progressFn func(current, total int)) error) (*model.JobStatus, error) {
	m.mu.RLock()
	for _, j := range m.jobs {
		if j.Type == jobType && j.Status == "running" {
			m.mu.RUnlock()
			return nil, fmt.Errorf("%s job already running", jobType)
		}
	}
	m.mu.RUnlock()

	job := &model.JobStatus{
		ID:        uuid.New().String(),
		Type:      jobType,
		Status:    "running",
		Progress:  "0/0",
		StartedAt: time.Now(),
	}

	m.mu.Lock()
	m.jobs[job.ID] = job
	m.mu.Unlock()

	go func() {
		progressFn := func(current, total int) {
			m.mu.Lock()
			job.Progress = fmt.Sprintf("%d/%d", current, total)
			m.mu.Unlock()
		}

		if err := fn(context.Background(), progressFn); err != nil {
			m.mu.Lock()
			job.Status = "failed"
			job.Error = err.Error()
			m.mu.Unlock()
			slog.Error("job failed", "job_id", job.ID, "type", jobType, "error", err)
			return
		}

		m.mu.Lock()
		job.Status = "completed"
		m.mu.Unlock()
		slog.Info("job completed", "job_id", job.ID, "type", jobType)
	}()

	return job, nil
}

func (m *JobManager) GetAll() []model.JobStatus {
	m.mu.RLock()
	defer m.mu.RUnlock()

	var jobs []model.JobStatus
	for _, j := range m.jobs {
		jobs = append(jobs, *j)
	}
	return jobs
}
