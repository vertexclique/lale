import PageMeta from "../../components/common/PageMeta";
import PageBreadcrumb from "../../components/common/PageBreadCrumb";
import TransactionList from "../../components/ecommerce/TransactionList";

export default function Transactions() {
  return (
    <>
      <PageMeta
        title="React.js E-commerce Single Invoice | LALE Prism - React.js Admin Dashboard Template"
        description="This is E-commerce React.js Single Invoice  page for LALE Prism - React.js Tailwind CSS Admin Dashboard Template"
      />
      <PageBreadcrumb pageTitle="Transactions" />
      <TransactionList />
    </>
  );
}
