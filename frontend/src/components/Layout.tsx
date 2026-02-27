import { Outlet, useParams } from 'react-router-dom'
import Sidebar from './Sidebar'
import FeedbackWidget from './FeedbackWidget'
import Banner from './Banner'
import Footer from './Footer'

interface LayoutProps {
  formContext?: boolean
  showSidebar?: boolean
}

export default function Layout({ formContext, showSidebar = true }: LayoutProps) {
  const { code } = useParams()

  return (
    <div className="min-h-screen h-screen overflow-hidden bg-gray-900 text-white flex flex-col">
      <Banner />
      <div className="flex flex-1 min-h-0 overflow-hidden">
        {showSidebar && <Sidebar />}
        <div className="flex flex-1 min-w-0 flex-col min-h-0">
          <main className="flex-1 min-w-0 overflow-auto">
            <Outlet context={formContext ? { formCode: code } : undefined} />
          </main>
          <Footer />
        </div>
        <FeedbackWidget />
      </div>
    </div>
  )
}
