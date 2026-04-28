import { Routes, Route } from 'react-router-dom'
import Layout from './components/Layout'
import IndexPage from './pages/IndexPage'
import InfoPage from './pages/InfoPage'
import CreateAccountPage from './pages/CreateAccountPage'
import ServersListPage from './pages/ServersListPage'
import ViewSchedulePage from './pages/ViewSchedulePage'
import SchedulesPage from './pages/SchedulesPage'
import StatsPage from './pages/StatsPage'
import AdminPage from './pages/AdminPage'
import DashboardPage from './pages/DashboardPage'
import AdminResourcesPage from './pages/AdminResourcesPage'
import AdminFeedbackPage from './pages/AdminFeedbackPage'
import AdminAllianceApplicationsPage from './pages/AdminAllianceApplicationsPage'
import FormPage from './pages/FormPage'
import FormStatsPage from './pages/FormStatsPage'
import TyrantFormPage from './pages/TyrantFormPage'

function App() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<IndexPage />} />
        <Route path="info" element={<InfoPage />} />
        <Route path="create-account" element={<CreateAccountPage />} />
        <Route path="servers" element={<ServersListPage />} />
        <Route path="view/:accountName/:server" element={<ViewSchedulePage />} />
        <Route path="dashboard/:accountName" element={<DashboardPage />} />
        <Route path="admin-resources" element={<AdminResourcesPage />} />
        <Route path="admin-alliance-applications" element={<AdminAllianceApplicationsPage />} />
        <Route path="admin-feedback" element={<AdminFeedbackPage />} />
        <Route path=":accountName/:formId" element={<SchedulesPage />} />
        <Route path=":accountName/:server/stats" element={<StatsPage />} />
        <Route path=":accountName/:server/admin" element={<AdminPage />} />
      </Route>
      <Route path="/form/:code" element={<Layout formContext showSidebar={false} />}>
        <Route index element={<FormPage />} />
        <Route path="stats" element={<FormStatsPage />} />
      </Route>
      <Route path="/tyrant-form/:code" element={<Layout formContext showSidebar={false} />}>
        <Route index element={<TyrantFormPage />} />
      </Route>
    </Routes>
  )
}

export default App
