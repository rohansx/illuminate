package model

type GrowthLevel string

const (
	LevelExplorer    GrowthLevel = "explorer"
	LevelFirstLight  GrowthLevel = "first_light"
	LevelContributor GrowthLevel = "contributor"
	LevelRegular     GrowthLevel = "regular"
	LevelSpecialist  GrowthLevel = "specialist"
	LevelLuminary    GrowthLevel = "luminary"
)

var LevelOrder = []GrowthLevel{
	LevelExplorer, LevelFirstLight, LevelContributor,
	LevelRegular, LevelSpecialist, LevelLuminary,
}

var LevelNames = map[GrowthLevel]string{
	LevelExplorer:    "Explorer",
	LevelFirstLight:  "First Light",
	LevelContributor: "Contributor",
	LevelRegular:     "Regular",
	LevelSpecialist:  "Specialist",
	LevelLuminary:    "Luminary",
}

type GrowthProfile struct {
	Level         GrowthLevel   `json:"level"`
	LevelName     string        `json:"level_name"`
	LevelIndex    int           `json:"level_index"`
	NextLevel     *GrowthLevel  `json:"next_level"`
	NextLevelName string        `json:"next_level_name"`
	Progress      LevelProgress `json:"progress"`
	Radar         RadarScores   `json:"radar"`
	NextSteps     []NextStep    `json:"next_steps"`
}

type LevelProgress struct {
	CurrentValue int    `json:"current_value"`
	TargetValue  int    `json:"target_value"`
	Metric       string `json:"metric"`
	Percentage   int    `json:"percentage"`
}

type RadarScores struct {
	Volume      int `json:"volume"`
	Breadth     int `json:"breadth"`
	Consistency int `json:"consistency"`
	Depth       int `json:"depth"`
	Diversity   int `json:"diversity"`
	Recency     int `json:"recency"`
}

type NextStep struct {
	ID          string `json:"id"`
	Title       string `json:"title"`
	Description string `json:"description"`
	Priority    int    `json:"priority"`
}
