import PageBreadcrumb from "../../components/common/PageBreadCrumb";
import PageMeta from "../../components/common/PageMeta";
import TaskListPage from "../../components/task/task-list/TaskListPage";

export default function TaskList() {
  return (
    <>
      <PageMeta
        title="React.js Task List Dashboard | LALE Prism - React.js Admin Dashboard Template"
        description="This is React.js Task List Dashboard page for LALE Prism - React.js Tailwind CSS Admin Dashboard Template"
      />
      <PageBreadcrumb pageTitle="Task List" />
      <TaskListPage />
    </>
  );
}
