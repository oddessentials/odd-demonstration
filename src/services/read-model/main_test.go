package main

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// ReadVersion reads the VERSION file and returns the version string.
func ReadVersion() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}

	versionPath := filepath.Join(dir, "VERSION")
	data, err := os.ReadFile(versionPath)
	if err != nil {
		return "", err
	}

	version := strings.TrimSpace(string(data))
	return version, nil
}

// IsValidSemVer checks if a version string matches SemVer format.
func IsValidSemVer(version string) bool {
	pattern := regexp.MustCompile(`^\d+\.\d+\.\d+$`)
	return pattern.MatchString(version)
}

// HealthResponse represents the health endpoint response.
type HealthResponse struct {
	Status  string `json:"status"`
	Version string `json:"version"`
}

// BuildHealthResponse creates a health response with version.
func BuildHealthResponse(version string) HealthResponse {
	return HealthResponse{
		Status:  "ok",
		Version: version,
	}
}

// MetricLabels includes version in metric labels.
func MetricLabels(service, version string) map[string]string {
	return map[string]string{
		"service": service,
		"version": version,
	}
}

// TestReadVersion tests that VERSION file returns valid SemVer.
func TestReadVersion(t *testing.T) {
	tmpDir := t.TempDir()
	versionPath := filepath.Join(tmpDir, "VERSION")
	err := os.WriteFile(versionPath, []byte("0.1.0\n"), 0644)
	if err != nil {
		t.Fatalf("Failed to create VERSION file: %v", err)
	}

	oldDir, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldDir)

	version, err := ReadVersion()
	if err != nil {
		t.Fatalf("ReadVersion failed: %v", err)
	}

	if !IsValidSemVer(version) {
		t.Errorf("Invalid SemVer format: %s", version)
	}

	if version != "0.1.0" {
		t.Errorf("Expected 0.1.0, got %s", version)
	}
}

// TestHealthResponseIncludesVersion tests health response contains version.
func TestHealthResponseIncludesVersion(t *testing.T) {
	version := "0.1.0"
	response := BuildHealthResponse(version)

	if response.Status != "ok" {
		t.Errorf("Expected status 'ok', got '%s'", response.Status)
	}

	if response.Version != version {
		t.Errorf("Expected version '%s', got '%s'", version, response.Version)
	}
}

// TestMetricLabelsIncludeVersion tests metric labels contain version.
func TestMetricLabelsIncludeVersion(t *testing.T) {
	labels := MetricLabels("read-model", "0.1.0")

	if labels["service"] != "read-model" {
		t.Errorf("Expected service 'read-model', got '%s'", labels["service"])
	}

	if labels["version"] != "0.1.0" {
		t.Errorf("Expected version '0.1.0', got '%s'", labels["version"])
	}
}
