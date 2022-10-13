function Is-Git($base) {
    $gitPath = Join-Path -Path $base -ChildPath .git
    if (Test-Path $gitPath) {
        $true
    } else {
        $next = Split-Path -Parent $base
        if ($next) {
            Is-Git $next
        } else {
            $false
        }
    }
}

function Repo-Path($base) {
    $gitPath = Join-Path -Path $base -ChildPath .git
    if (Test-Path $gitPath) {
        Split-Path -Path $base -Leaf
    } else {
        $next = Split-Path -Parent $base
        if ($next) {
            Join-Path -Path $(Repo-Path $next) -ChildPath $(Split-Path -Path $base -Leaf)
        } else {
            ""
        }
    }
}

function prompt {
    $loc = Get-Location
    $isGit = Is-Git $loc

    $repoPath = Repo-Path $loc

    if ($isGit) {
        Write-Host $repoPath -ForegroundColor "blue" -NoNewline
        Write-Host $(" (" + $(git branch --show-current) + ")") -ForegroundColor "green"
        "$ "
    } else {
        $(Get-Location) + "`n$ "
    }
}
