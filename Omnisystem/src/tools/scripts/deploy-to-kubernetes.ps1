#!/usr/bin/env pwsh
<#
.SYNOPSIS
Deploy Bonsai to Kubernetes cluster

.DESCRIPTION
Orchestrates production deployment with canary rollouts, health checks, and rollback

.PARAMETER Cluster
Kubernetes cluster name

.PARAMETER Namespace
Namespace to deploy to (default: bonsai)

.PARAMETER Image
Container image to deploy

.PARAMETER Canary
Use canary deployment (default: true)

.PARAMETER CanaryWeight
Initial canary weight percentage (default: 5)

.PARAMETER WaitForReady
Wait for rollout to complete (default: true)

.PARAMETER DryRun
Show what would happen without applying
#>

param(
    [Parameter(Mandatory = $true)][string]$Cluster,
    [string]$Namespace = "bonsai",
    [Parameter(Mandatory = $true)][string]$Image,
    [switch]$Canary = $true,
    [int]$CanaryWeight = 5,
    [switch]$WaitForReady = $true,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$colors = @{
    success = "Green"
    error = "Red"
    warning = "Yellow"
    info = "Cyan"
}

function Write-Log {
    param([string]$Message, [string]$Type = "info")
    $color = $colors[$Type]
    Write-Host "[$((Get-Date).ToString('HH:mm:ss'))] $Message" -ForegroundColor $color
}

function Test-Prerequisites {
    Write-Log "Validating prerequisites..." "info"

    try {
        kubectl version --client | Out-Null
        Write-Log "✓ kubectl available" "success"
    }
    catch {
        Write-Log "✗ kubectl not found" "error"
        throw "kubectl required for deployment"
    }
}

function Connect-Cluster {
    Write-Log "Connecting to cluster: $Cluster" "info"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would connect to: $Cluster" "warning"
    }
    else {
        # This assumes cluster credentials are configured
        # Adjust based on your cluster setup (EKS, GKE, AKS, etc.)
        try {
            kubectl cluster-info | Out-Null
            Write-Log "✓ Connected to cluster" "success"
        }
        catch {
            Write-Log "✗ Cannot connect to cluster" "error"
            throw "Failed to connect to cluster"
        }
    }
}

function Create-Namespace {
    Write-Log "Ensuring namespace exists: $Namespace" "info"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would create namespace: $Namespace" "warning"
    }
    else {
        kubectl create namespace $Namespace --dry-run=client -o yaml | kubectl apply -f -
        Write-Log "✓ Namespace ready" "success"
    }
}

function Update-Image {
    Write-Log "Updating deployment image: $Image" "info"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would update image to: $Image" "warning"
    }
    else {
        kubectl set image deployment/bonsai-core `
            -n $Namespace `
            bonsai-core=$Image `
            --record

        Write-Log "✓ Image updated" "success"
    }
}

function Deploy-Manifests {
    Write-Log "Applying Kubernetes manifests..." "info"

    $manifests = @(
        "deploy/kubernetes/bonsai-deployment.yaml"
    )

    if ($Canary) {
        $manifests += "deploy/kubernetes/bonsai-canary-deployment.yaml"
    }

    foreach ($manifest in $manifests) {
        if (-not (Test-Path $manifest)) {
            Write-Log "⚠ Manifest not found: $manifest" "warning"
            continue
        }

        if ($DryRun) {
            Write-Log "[DRY RUN] Would apply: $manifest" "warning"
        }
        else {
            kubectl apply -f $manifest
            Write-Log "✓ Applied: $manifest" "success"
        }
    }
}

function Wait-Rollout {
    Write-Log "Waiting for rollout to complete..." "info"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would wait for rollout" "warning"
        return
    }

    if (-not $WaitForReady) {
        Write-Log "Skipping wait for rollout" "warning"
        return
    }

    try {
        kubectl rollout status deployment/bonsai-core `
            -n $Namespace `
            --timeout=10m

        Write-Log "✓ Rollout complete" "success"
    }
    catch {
        Write-Log "✗ Rollout failed or timeout" "error"
        throw "Deployment did not reach ready state"
    }
}

function Check-Health {
    Write-Log "Checking deployment health..." "info"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would check health" "warning"
        return
    }

    # Wait for service endpoint
    Start-Sleep -Seconds 5

    try {
        $svc = kubectl get service bonsai-core -n $Namespace -o json | ConvertFrom-Json
        $ip = $svc.status.loadBalancer.ingress[0].ip

        if ([string]::IsNullOrEmpty($ip)) {
            $ip = $svc.spec.clusterIP
        }

        Write-Log "Service IP: $ip" "info"

        # Health check
        $attempts = 0
        while ($attempts -lt 10) {
            try {
                $response = curl -s "http://$ip:8082/health/live" -w "`n%{http_code}"
                $status = ($response | Select-Object -Last 1)

                if ($status -eq "200") {
                    Write-Log "✓ Health check passed" "success"
                    return
                }
            }
            catch {
                # Expected if service not ready yet
            }

            $attempts++
            Start-Sleep -Seconds 3
        }

        Write-Log "⚠ Health check timeout, but continuing" "warning"
    }
    catch {
        Write-Log "⚠ Could not perform health check: $_" "warning"
    }
}

function Get-Deployment-Status {
    Write-Log "Deployment Status" "info"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would show deployment status" "warning"
        return
    }

    Write-Host "`nDeployment:" -ForegroundColor Cyan
    kubectl get deployment bonsai-core -n $Namespace

    Write-Host "`nPods:" -ForegroundColor Cyan
    kubectl get pods -n $Namespace -l app=bonsai,component=core

    Write-Host "`nReplicaSet:" -ForegroundColor Cyan
    kubectl get rs -n $Namespace -l app=bonsai,component=core

    Write-Host "`nService:" -ForegroundColor Cyan
    kubectl get svc bonsai-core -n $Namespace

    Write-Host "`nEvents:" -ForegroundColor Cyan
    kubectl get events -n $Namespace --sort-by='.lastTimestamp' | tail -10
}

function Rollback-Deployment {
    param([string]$Revision)

    Write-Log "Rolling back deployment..." "error"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would rollback deployment" "warning"
        return
    }

    if ([string]::IsNullOrEmpty($Revision)) {
        kubectl rollout undo deployment/bonsai-core -n $Namespace
    }
    else {
        kubectl rollout undo deployment/bonsai-core -n $Namespace --to-revision=$Revision
    }

    Write-Log "✓ Rollback initiated" "success"

    # Wait for rollback
    kubectl rollout status deployment/bonsai-core -n $Namespace --timeout=5m
    Write-Log "✓ Rollback complete" "success"
}

function Create-Backup {
    Write-Log "Creating deployment backup..." "info"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would create backup" "warning"
        return
    }

    $timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
    $backupFile = "deploy/backups/bonsai-deployment-$timestamp.yaml"

    $backupDir = Split-Path $backupFile
    if (-not (Test-Path $backupDir)) {
        New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
    }

    kubectl get all -n $Namespace -o yaml > $backupFile
    Write-Log "✓ Backup created: $backupFile" "success"
}

# Main execution
try {
    Write-Log "Starting Bonsai Kubernetes deployment..." "info"
    Write-Log "Cluster: $Cluster, Namespace: $Namespace, Image: $Image" "info"
    if ($DryRun) { Write-Log "DRY RUN MODE" "warning" }

    Test-Prerequisites
    Connect-Cluster
    Create-Namespace
    Create-Backup

    Deploy-Manifests
    Update-Image

    if ($WaitForReady) {
        Wait-Rollout
        Check-Health
    }

    Get-Deployment-Status

    Write-Log "✓ Deployment successful" "success"
}
catch {
    Write-Log "✗ Deployment failed: $_" "error"

    if (-not $DryRun) {
        Write-Log "Attempting rollback..." "warning"
        try {
            Rollback-Deployment
        }
        catch {
            Write-Log "✗ Rollback also failed. Manual intervention required." "error"
        }
    }

    exit 1
}
