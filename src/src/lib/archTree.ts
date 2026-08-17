import type { WorkspaceIndex } from "../types";

export interface ArchitectureBranch {
  id: string;
  name: string;
  category: "core" | "context" | "agents" | "llm" | "fs" | "ui" | "network" | "templates";
  fileCount: number;
  totalSizeBytes: number;
  files: string[];
  isActive: boolean;
  color: string;
  glowColor: string;
  pathCoordinate: {
    startX: number;
    startY: number;
    endX: number;
    endY: number;
    controlX: number;
    controlY: number;
  };
}

export interface ArchitectureTreeData {
  branches: ArchitectureBranch[];
  totalFiles: number;
  activeBranchId: string | null;
}

export function extractArchitectureTree(
  workspace: WorkspaceIndex | null,
  activeTaskTarget?: string | null,
  activeFilePath?: string | null
): ArchitectureTreeData {
  const branchMap: Record<
    string,
    {
      name: string;
      category: ArchitectureBranch["category"];
      files: string[];
      totalSize: number;
      color: string;
      glowColor: string;
      pathCoordinate: ArchitectureBranch["pathCoordinate"];
    }
  > = {
    core: {
      name: "Core / Types",
      category: "core",
      files: [],
      totalSize: 0,
      color: "#a855f7", // Violet
      glowColor: "rgba(168, 85, 247, 0.6)",
      pathCoordinate: { startX: 100, startY: 150, endX: 35, endY: 80, controlX: 65, controlY: 110 },
    },
    context: {
      name: "Context / AST",
      category: "context",
      files: [],
      totalSize: 0,
      color: "#06b6d4", // Cyan
      glowColor: "rgba(6, 182, 212, 0.6)",
      pathCoordinate: { startX: 100, startY: 150, endX: 55, endY: 40, controlX: 75, controlY: 85 },
    },
    agents: {
      name: "Agents / DAG",
      category: "agents",
      files: [],
      totalSize: 0,
      color: "#10b981", // Emerald
      glowColor: "rgba(16, 185, 129, 0.6)",
      pathCoordinate: { startX: 100, startY: 150, endX: 100, endY: 25, controlX: 100, controlY: 70 },
    },
    llm: {
      name: "LLM / Routing",
      category: "llm",
      files: [],
      totalSize: 0,
      color: "#f59e0b", // Amber
      glowColor: "rgba(245, 158, 11, 0.6)",
      pathCoordinate: { startX: 100, startY: 150, endX: 145, endY: 40, controlX: 125, controlY: 85 },
    },
    fs: {
      name: "FS / Diff Engine",
      category: "fs",
      files: [],
      totalSize: 0,
      color: "#3b82f6", // Blue
      glowColor: "rgba(59, 130, 246, 0.6)",
      pathCoordinate: { startX: 100, startY: 150, endX: 165, endY: 80, controlX: 135, controlY: 110 },
    },
    ui: {
      name: "UI / React",
      category: "ui",
      files: [],
      totalSize: 0,
      color: "#ec4899", // Pink
      glowColor: "rgba(236, 72, 153, 0.6)",
      pathCoordinate: { startX: 100, startY: 150, endX: 175, endY: 125, controlX: 140, controlY: 135 },
    },
    network: {
      name: "P2P Network",
      category: "network",
      files: [],
      totalSize: 0,
      color: "#8b5cf6", // Purple
      glowColor: "rgba(139, 92, 246, 0.6)",
      pathCoordinate: { startX: 100, startY: 150, endX: 25, endY: 125, controlX: 60, controlY: 135 },
    },
  };

  let totalFiles = 0;

  if (workspace?.files) {
    for (const [filePath, file] of Object.entries(workspace.files)) {
      totalFiles++;
      const lower = filePath.toLowerCase();

      if (lower.includes("locus-core") || lower.includes("/types.") || lower.includes("config")) {
        branchMap.core.files.push(filePath);
        branchMap.core.totalSize += file.size;
      } else if (lower.includes("locus-context") || lower.includes("ast") || lower.includes("semantic") || lower.includes("skeleton")) {
        branchMap.context.files.push(filePath);
        branchMap.context.totalSize += file.size;
      } else if (lower.includes("locus-agents") || lower.includes("agent") || lower.includes("task_graph") || lower.includes("reasoning") || lower.includes("skill")) {
        branchMap.agents.files.push(filePath);
        branchMap.agents.totalSize += file.size;
      } else if (lower.includes("locus-llm") || lower.includes("keyring") || lower.includes("router") || lower.includes("provider")) {
        branchMap.llm.files.push(filePath);
        branchMap.llm.totalSize += file.size;
      } else if (lower.includes("locus-fs") || lower.includes("diff") || lower.includes("snapshot") || lower.includes("search_replace")) {
        branchMap.fs.files.push(filePath);
        branchMap.fs.totalSize += file.size;
      } else if (lower.includes("src/") || lower.includes(".tsx") || lower.includes(".css") || lower.includes("components/")) {
        branchMap.ui.files.push(filePath);
        branchMap.ui.totalSize += file.size;
      } else if (lower.includes("locus-network") || lower.includes("p2p") || lower.includes("discovery")) {
        branchMap.network.files.push(filePath);
        branchMap.network.totalSize += file.size;
      } else {
        branchMap.core.files.push(filePath);
        branchMap.core.totalSize += file.size;
      }
    }
  }

  // Determine active branch based on activeTaskTarget or activeFilePath
  let activeBranchId: string | null = null;
  const activeTargetLower = (activeTaskTarget || activeFilePath || "").toLowerCase();

  if (activeTargetLower) {
    if (activeTargetLower.includes("context") || activeTargetLower.includes("skeleton") || activeTargetLower.includes("ast")) {
      activeBranchId = "context";
    } else if (activeTargetLower.includes("agent") || activeTargetLower.includes("task") || activeTargetLower.includes("reasoning") || activeTargetLower.includes("skill")) {
      activeBranchId = "agents";
    } else if (activeTargetLower.includes("llm") || activeTargetLower.includes("keyring") || activeTargetLower.includes("router")) {
      activeBranchId = "llm";
    } else if (activeTargetLower.includes("fs") || activeTargetLower.includes("diff") || activeTargetLower.includes("search_replace")) {
      activeBranchId = "fs";
    } else if (activeTargetLower.includes("tsx") || activeTargetLower.includes("components") || activeTargetLower.includes("ui")) {
      activeBranchId = "ui";
    } else if (activeTargetLower.includes("network")) {
      activeBranchId = "network";
    } else if (activeTargetLower.includes("core")) {
      activeBranchId = "core";
    }
  }

  // Default to agents or core if active task is running
  if (!activeBranchId && activeTaskTarget) {
    activeBranchId = "agents";
  }

  const branches: ArchitectureBranch[] = Object.entries(branchMap).map(
    ([id, data]) => ({
      id,
      name: data.name,
      category: data.category,
      fileCount: data.files.length,
      totalSizeBytes: data.totalSize,
      files: data.files,
      isActive: activeBranchId === id,
      color: data.color,
      glowColor: data.glowColor,
      pathCoordinate: data.pathCoordinate,
    })
  );

  return {
    branches,
    totalFiles,
    activeBranchId,
  };
}
