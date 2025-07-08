import { useState } from "react";
import { 
  Button, 
  Card, 
  CardHeader, 
  CardTitle, 
  CardDescription, 
  CardContent,
  StatusBadge,
  TechStackBadge,
  ScoreBadge,
  ProjectTypeBadge,
  Modal,
  ModalHeader,
  ModalTitle,
  ModalDescription,
  ModalFooter,
  PageLoading,
  InlineLoading,
  Input
} from "@/components/ui";
import "./App.css";

// Mock data based on our manual review experience
const mockProjects = [
  {
    id: 1,
    studentName: "Geoffrey Wachira Gichuki",
    projectTitle: "AI Poster Maker Backend",
    githubUrl: "https://github.com/geoffeycloud/poster-maker-backend",
    technology: ["FastAPI", "Python", "AI"],
    score: 99,
    status: "completed" as const,
    type: "individual" as const,
    feedback: "Outstanding enterprise-grade AI poster generation backend with FastAPI and Google Cloud Vertex AI...",
    lastUpdated: "2 hours ago"
  },
  {
    id: 2,
    studentName: "Omollo Victor",
    projectTitle: "STAFF_PULSE",
    githubUrl: "https://github.com/SK3CHI3/STAFF_PULSE.git",
    technology: ["React", "Next.js", "TypeScript"],
    score: 98,
    status: "completed" as const,
    type: "individual" as const,
    feedback: "Outstanding enterprise-grade employee wellness platform with WhatsApp integration...",
    lastUpdated: "3 hours ago"
  },
  {
    id: 3,
    studentName: "James Muganzi Imoli",
    projectTitle: "Ajirawise Job Alert Bot",
    githubUrl: "https://github.com/MuganziJames/-whatsapp-job-alert-bot-kenya.git",
    technology: ["Python", "Flask", "WhatsApp Bot"],
    score: 97,
    status: "completed" as const,
    type: "team" as const,
    feedback: "OUTSTANDING professional-grade dual-platform job alert system for Kenyan youth...",
    lastUpdated: "4 hours ago"
  },
  {
    id: 4,
    studentName: "Linus Kipkemoi Langat",
    projectTitle: "RecycloHub AI Waste Management",
    githubUrl: "https://github.com/Joy-kitet/Hackathon-2.0",
    technology: ["React", "FastAPI", "AI"],
    score: 96,
    status: "in-progress" as const,
    type: "team" as const,
    feedback: "OUTSTANDING enterprise-grade AI waste management platform...",
    lastUpdated: "1 hour ago"
  },
  {
    id: 5,
    studentName: "Patricia Nduku Mugisia",
    projectTitle: "Haki-kenya Legal Aid Platform",
    githubUrl: "https://github.com/Patricianduku/Haki-kenya.git",
    technology: ["React", "TypeScript", "Supabase"],
    score: 95,
    status: "completed" as const,
    type: "individual" as const,
    feedback: "Exceptional React legal aid platform with Supabase backend...",
    lastUpdated: "5 hours ago"
  },
  {
    id: 6,
    studentName: "Magdaline Mutave",
    projectTitle: "Poster & Flier Maker",
    githubUrl: "https://github.com/DOMOSH85/poster-flier-maker.git",
    technology: ["React", "Node.js", "MongoDB"],
    score: 93,
    status: "pending" as const,
    type: "team" as const,
    feedback: "EXCEPTIONAL commercial-grade full-stack design platform...",
    lastUpdated: "30 minutes ago"
  }
];

function App() {
  const [selectedProject, setSelectedProject] = useState<typeof mockProjects[0] | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");
  const [filterStatus, setFilterStatus] = useState<"all" | "pending" | "in-progress" | "completed">("all");

  const filteredProjects = mockProjects.filter(project => {
    const matchesSearch = project.studentName.toLowerCase().includes(searchTerm.toLowerCase()) ||
                         project.projectTitle.toLowerCase().includes(searchTerm.toLowerCase());
    const matchesStatus = filterStatus === "all" || project.status === filterStatus;
    return matchesSearch && matchesStatus;
  });

  const handleAnalyzeProject = async (projectId: number) => {
    setIsAnalyzing(true);
    // Simulate analysis process
    await new Promise(resolve => setTimeout(resolve, 3000));
    setIsAnalyzing(false);
    
    // Update project status to completed
    const project = mockProjects.find(p => p.id === projectId);
    if (project) {
      project.status = "completed";
    }
  };

  const stats = {
    total: mockProjects.length,
    completed: mockProjects.filter(p => p.status === "completed").length,
    inProgress: mockProjects.filter(p => p.status === "in-progress").length,
    pending: mockProjects.filter(p => p.status === "pending").length,
    averageScore: Math.round(mockProjects.reduce((acc, p) => acc + p.score, 0) / mockProjects.length)
  };

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <header className="bg-white dark:bg-gray-800 shadow-sm border-b">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center">
              <h1 className="text-2xl font-bold text-gray-900 dark:text-white">r3viewer</h1>
              <span className="ml-2 text-sm text-gray-500">Student Project Review System</span>
            </div>
            <div className="flex items-center space-x-4">
              <Button variant="outline" size="sm">
                Import from Sheets
              </Button>
              <Button size="sm">
                Analyze All
              </Button>
            </div>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-5 gap-6 mb-8">
          <Card>
            <CardContent className="p-6">
              <div className="text-2xl font-bold">{stats.total}</div>
              <p className="text-sm text-muted-foreground">Total Projects</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6">
              <div className="text-2xl font-bold text-green-600">{stats.completed}</div>
              <p className="text-sm text-muted-foreground">Completed</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6">
              <div className="text-2xl font-bold text-yellow-600">{stats.inProgress}</div>
              <p className="text-sm text-muted-foreground">In Progress</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6">
              <div className="text-2xl font-bold text-gray-600">{stats.pending}</div>
              <p className="text-sm text-muted-foreground">Pending</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6">
              <div className="text-2xl font-bold text-blue-600">{stats.averageScore}%</div>
              <p className="text-sm text-muted-foreground">Avg Score</p>
            </CardContent>
          </Card>
        </div>

        {/* Filters */}
        <div className="flex flex-col sm:flex-row gap-4 mb-6">
          <div className="flex-1">
            <Input
              placeholder="Search projects or students..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full"
            />
          </div>
          <div className="flex gap-2">
            {["all", "pending", "in-progress", "completed"].map((status) => (
              <Button
                key={status}
                variant={filterStatus === status ? "default" : "outline"}
                size="sm"
                onClick={() => setFilterStatus(status as any)}
              >
                {status.charAt(0).toUpperCase() + status.slice(1).replace("-", " ")}
              </Button>
            ))}
          </div>
        </div>

        {/* Projects Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
          {filteredProjects.map((project) => (
            <Card key={project.id} className="hover:shadow-lg transition-shadow cursor-pointer">
              <CardHeader>
                <div className="flex justify-between items-start">
                  <div className="flex-1">
                    <CardTitle className="text-lg mb-1">{project.projectTitle}</CardTitle>
                    <CardDescription>{project.studentName}</CardDescription>
                  </div>
                  <ScoreBadge score={project.score} />
                </div>
                <div className="flex flex-wrap gap-2 mt-3">
                  <StatusBadge status={project.status} size="sm" />
                  <ProjectTypeBadge type={project.type} size="sm" />
                </div>
              </CardHeader>
              <CardContent>
                <div className="flex flex-wrap gap-1 mb-4">
                  {project.technology.map((tech) => (
                    <TechStackBadge key={tech} technology={tech} size="sm" />
                  ))}
                </div>
                <p className="text-sm text-muted-foreground mb-4 line-clamp-3">
                  {project.feedback}
                </p>
                <div className="flex justify-between items-center">
                  <span className="text-xs text-muted-foreground">{project.lastUpdated}</span>
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => setSelectedProject(project)}
                    >
                      View Details
                    </Button>
                    {project.status === "pending" && (
                      <Button
                        size="sm"
                        onClick={() => handleAnalyzeProject(project.id)}
                        disabled={isAnalyzing}
                      >
                        {isAnalyzing ? <InlineLoading text="Analyzing..." /> : "Analyze"}
                      </Button>
                    )}
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>

        {filteredProjects.length === 0 && (
          <div className="text-center py-12">
            <div className="text-gray-500 text-lg">No projects found matching your criteria</div>
            <p className="text-gray-400 mt-2">Try adjusting your search or filters</p>
          </div>
        )}
      </main>

      {/* Project Detail Modal */}
      <Modal
        isOpen={!!selectedProject}
        onClose={() => setSelectedProject(null)}
        title="Project Details"
        size="xl"
      >
        {selectedProject && (
          <div className="space-y-6">
            <ModalHeader>
              <ModalTitle>{selectedProject.projectTitle}</ModalTitle>
              <ModalDescription>by {selectedProject.studentName}</ModalDescription>
            </ModalHeader>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <h4 className="font-semibold mb-2">Status & Score</h4>
                <div className="flex gap-2">
                  <StatusBadge status={selectedProject.status} />
                  <ScoreBadge score={selectedProject.score} />
                  <ProjectTypeBadge type={selectedProject.type} />
                </div>
              </div>
              <div>
                <h4 className="font-semibold mb-2">Technology Stack</h4>
                <div className="flex flex-wrap gap-1">
                  {selectedProject.technology.map((tech) => (
                    <TechStackBadge key={tech} technology={tech} />
                  ))}
                </div>
              </div>
            </div>

            <div>
              <h4 className="font-semibold mb-2">Repository</h4>
              <a 
                href={selectedProject.githubUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-600 hover:underline break-all"
              >
                {selectedProject.githubUrl}
              </a>
            </div>

            <div>
              <h4 className="font-semibold mb-2">Feedback</h4>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {selectedProject.feedback}
              </p>
            </div>

            <ModalFooter>
              <Button variant="outline" onClick={() => setSelectedProject(null)}>
                Close
              </Button>
              <Button>
                Open in Playground
              </Button>
            </ModalFooter>
          </div>
        )}
      </Modal>

      {/* Loading Overlay */}
      {isAnalyzing && (
        <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-40">
          <PageLoading text="Analyzing project... This may take a few minutes." />
        </div>
      )}
    </div>
  );
}

export default App;
